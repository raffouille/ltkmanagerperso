use super::*;

fn skip(line: &str) -> String {
    let line: Vec<u16> = line.encode_utf16().collect();
    String::from_utf16(skip_program_name(&line)).unwrap()
}

#[test]
fn skip_program_name_follows_command_line_to_argv() {
    assert_eq!(
        skip(r#""C:\Program Files\LTK\ltk-manager.exe" "ltk://x y" a"#),
        r#""ltk://x y" a"#
    );
    assert_eq!(skip(r"C:\ltk\ltk-manager.exe   a b"), "a b");
    assert_eq!(skip(r#""C:\ltk\ltk-manager.exe""#), "");
    assert_eq!(skip("ltk-manager.exe"), "");
}
