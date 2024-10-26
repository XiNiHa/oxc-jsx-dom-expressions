use oxc::span::Atom;

pub fn jsx_text_to_str(t: &Atom) -> String {
    let mut buf = String::new();
    let replaced = t.replace('\r', "").replace('\t', " ");
    let mut lines = replaced.lines().enumerate().peekable();

    while let Some((i, mut line)) = lines.next() {
        if line.is_empty() {
            continue;
        }
        if i != 0 {
            line = line.trim_start_matches(' ');
        }
        if lines.peek().is_some() {
            line = line.trim_end_matches(' ');
        }
        if line.is_empty() {
            continue;
        }
        if i != 0 && !buf.is_empty() {
            buf.push(' ')
        }
        buf.push_str(line);
    }
    buf
}
