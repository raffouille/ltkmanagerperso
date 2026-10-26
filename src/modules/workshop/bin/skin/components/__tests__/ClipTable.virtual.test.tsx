// @vitest-environment happy-dom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { AnimationGraph, GraphClip } from "@/lib/tauri";

import { ROW_HEIGHT } from "../../../tree/components/BinRow";
import { skinQueries } from "../../api/skinQueries";
import { SkinChoiceContext, useSkinChoice } from "../../state/skinChoice";
import { ClipsPane, ClipTabs } from "../ClipTable";

const { draw, open } = vi.hoisted(() => ({ draw: vi.fn(), open: vi.fn() }));

vi.mock("../../../../state", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../../../state")>()),
  useOpenDocumentAs: () => open,
}));

vi.mock("../ClipColumns", () => ({
  shownColumns: () => [
    {
      key: "name",
      width: "w-64",
      label: () => "Name",
      value: (clip: GraphClip) => clip.name,
      draw: (clip: GraphClip) => {
        draw(clip.hash);
        return clip.name;
      },
    },
    {
      key: "events",
      width: "w-16",
      label: () => "Events",
      value: (clip: GraphClip) => clip.events.length,
      draw: (clip: GraphClip) => clip.events.length,
    },
  ],
}));
vi.mock("../ClipDetail", () => ({ ClipDetail: () => <div>Clip details</div> }));
vi.mock("@tanstack/react-virtual", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@tanstack/react-virtual")>();
  return {
    ...actual,
    useVirtualizer: (
      options: Parameters<typeof actual.useVirtualizer<HTMLDivElement, HTMLDivElement>>[0],
    ) =>
      actual.useVirtualizer({
        ...options,
        observeElementRect: (_instance, callback) => {
          callback({ width: 800, height: 240 });
        },
        measureElement: () => ROW_HEIGHT,
      }),
  };
});

const clips: GraphClip[] = Array.from({ length: 5000 }, (_, index) => ({
  name: `Clip${String(index).padStart(4, "0")}`,
  hash: String(index),
  class: "AtomicClipData",
  animation: { path: `test${index}.anm`, asset: { kind: "file", path: `test${index}.anm` } },
  track: null,
  mask: null,
  syncGroup: null,
  tickDuration: null,
  events: new Array<GraphClip["events"][number]>(index % 13),
  children: [],
  parameters: [],
  interruptionGroups: [],
  flags: 0,
  ownEvents: false,
}));

function Harness() {
  const choice = useSkinChoice();
  return (
    <SkinChoiceContext value={choice}>
      <ClipTabs />
      <ClipsPane source={{ document: 1, graph: "graph" }} joints={null} />
    </SkinChoiceContext>
  );
}

function mount() {
  const client = new QueryClient();
  const graph: AnimationGraph = { clips, tracks: [], masks: [], syncGroups: [] };
  client.setQueryData(skinQueries.graph(1, "graph").queryKey, graph);
  render(
    <QueryClientProvider client={client}>
      <Harness />
    </QueryClientProvider>,
  );
  const scroller = screen.getByRole("table").parentElement!.parentElement!;
  return scroller;
}

/** The collapsed clip rows, apart from the fold caret each one starts with. */
function clipRows() {
  return screen
    .getAllByRole("button", { expanded: false })
    .filter((each) => each.getAttribute("role") === "button");
}

afterEach(() => {
  vi.useRealTimers();
  vi.clearAllMocks();
});

describe("clip virtualization", () => {
  it("opens the animation of the right-clicked clip without changing the playing clip", async () => {
    mount();
    const first = screen.getByText("Clip0000").closest('[role="button"]')!;
    const second = screen.getByText("Clip0001").closest('[role="button"]')!;
    fireEvent.click(first);
    fireEvent.contextMenu(second);
    fireEvent.click(await screen.findByRole("menuitem", { name: /^Open animation$/ }));
    expect(first).toHaveAttribute("aria-pressed", "true");
    expect(open).toHaveBeenCalledTimes(1);
    expect(JSON.stringify(open.mock.calls[0]?.[0])).toContain("test1.anm");
    expect(open.mock.calls[0]?.[1]).toBe("default");
  });

  it("sorts names and numeric columns across the entire virtualized graph", () => {
    mount();
    fireEvent.click(screen.getByRole("button", { name: "Name" }));
    expect(screen.getByRole("columnheader", { name: "Name" })).toHaveAttribute(
      "aria-sort",
      "descending",
    );
    expect(screen.getByText("Clip4999")).toBeInTheDocument();
    expect(screen.queryByText("Clip0000")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Events" }));
    expect(screen.getByRole("columnheader", { name: "Events" })).toHaveAttribute(
      "aria-sort",
      "ascending",
    );
    const values = () => clipRows().map((row) => Number(row.lastElementChild?.textContent));
    expect(values().every((value) => value === 0)).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Events" }));
    expect(values().every((value) => value === 12)).toBe(true);
  });

  it("windows a large graph and reuses cells that remain visible while scrolling", () => {
    const scroller = mount();
    expect(screen.getByText("Clip0000")).toBeInTheDocument();
    expect(screen.queryByText("Clip4999")).not.toBeInTheDocument();
    expect(draw.mock.calls.length).toBeLessThan(40);
    draw.mockClear();
    /* The virtualizer ends a scroll on a timer that outlives the file, and its update throws
       once the environment is torn down. */
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    act(() => {
      scroller.scrollTop = ROW_HEIGHT;
      fireEvent.scroll(scroller);
    });
    expect(draw.mock.calls.some(([hash]) => hash === "1")).toBe(false);
    expect(draw.mock.calls.length).toBeLessThan(5);
    act(() => {
      scroller.scrollTop = ROW_HEIGHT * 1000;
      fireEvent.scroll(scroller);
    });
    expect(screen.getByText("Clip1000")).toBeInTheDocument();
    expect(screen.queryByText("Clip0000")).not.toBeInTheDocument();
    expect(clipRows().length).toBeLessThan(40);
  });

  it("updates the playing indicator inside reused cells and expands the selected row", () => {
    mount();
    const first = screen.getByText("Clip0000").closest('[role="button"]')!;
    const second = screen.getByText("Clip0001").closest('[role="button"]')!;
    fireEvent.click(first);
    expect(within(first as HTMLElement).getByRole("img")).toBeInTheDocument();
    fireEvent.click(second);
    expect(within(first as HTMLElement).queryByRole("img")).not.toBeInTheDocument();
    expect(within(second as HTMLElement).getByRole("img")).toBeInTheDocument();
    fireEvent.keyDown(second, { key: "ArrowRight" });
    expect(screen.getByText("Clip details")).toBeInTheDocument();
    expect(second).toHaveAttribute("aria-expanded", "true");
  });
});
