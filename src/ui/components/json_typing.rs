//! JSON typing assistance expressed as ordinary TextEdit events so undo stays native.
use eframe::egui::{text::CCursorRange, Event, ImeEvent, Key, Modifiers};

fn arrow(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

pub(super) fn events(text: &str, cursor: CCursorRange, events: &[Event]) -> Vec<Event> {
    // Leave an in-progress composition to TextEdit. Linux input methods also
    // deliver ordinary punctuation as Commit, which must receive typing assistance.
    let commits: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::Ime(ImeEvent::Commit(value)) => Some(value.as_str()),
            _ => None,
        })
        .collect();
    let completed_delimiter = commits.len() == 1
        && matches!(commits[0], "{" | "}" | "\"")
        && !events
            .iter()
            .any(|event| matches!(event, Event::Text(_) | Event::Paste(_) | Event::Cut))
        && !events
            .iter()
            .rev()
            .find(|event| {
                matches!(
                    event,
                    Event::Ime(ImeEvent::Preedit(_) | ImeEvent::Commit(_))
                )
            })
            .is_some_and(|event| matches!(event, Event::Ime(ImeEvent::Preedit(_))));
    if !completed_delimiter
        && events
            .iter()
            .any(|event| matches!(event, Event::Ime(ImeEvent::Preedit(_))))
    {
        return events.to_vec();
    }
    let events: Vec<Event> = events
        .iter()
        .flat_map(|event| {
            match event {
                // A completed composition replaces the current selection directly.
                Event::Ime(ImeEvent::Preedit(_)) if completed_delimiter => Vec::new(),
                Event::Ime(ImeEvent::Commit(value))
                    if matches!(value.as_str(), "{" | "}" | "\"") =>
                {
                    // Finish composition before inserting at its current selection.
                    // TextEdit then handles both the text and synthetic cursor movement.
                    vec![Event::Ime(ImeEvent::Disabled), Event::Text(value.clone())]
                }
                _ => vec![event.clone()],
            }
        })
        .collect();
    let mut chars: Vec<char> = text.chars().collect();
    let mut start = cursor
        .primary
        .index
        .min(cursor.secondary.index)
        .min(chars.len());
    let mut end = cursor
        .primary
        .index
        .max(cursor.secondary.index)
        .min(chars.len());
    let mut result = Vec::new();
    let mut can_assist = true;
    for event in &events {
        match event {
            Event::Text(insert) | Event::Paste(insert) if can_assist => {
                let mut quoted = false;
                let mut escaped = false;
                for &ch in &chars[..start] {
                    if escaped {
                        escaped = false;
                    } else if quoted && ch == '\\' {
                        escaped = true;
                    } else if ch == '"' {
                        quoted = !quoted;
                    }
                }
                let next = chars.get(end).copied();
                let single = matches!(event, Event::Text(_)) && insert.chars().count() == 1;
                let ch = insert.chars().next().unwrap_or('\0');
                let skip = single
                    && start == end
                    && next == Some(ch)
                    && ((ch == '}' && !quoted) || (ch == '"' && quoted && !escaped));
                let boundary =
                    next.is_none_or(|c| c.is_whitespace() || matches!(c, ',' | ':' | '}' | ']'));
                let close = if single && !quoted && boundary {
                    match ch {
                        '{' => Some('}'),
                        '"' => Some('"'),
                        _ => None,
                    }
                } else {
                    None
                };
                if single && ch == '"' && start < end {
                    let selected: String = chars[start..end].iter().collect();
                    let wrapped = format!("\"{selected}\"");
                    result.push(Event::Text(wrapped.clone()));
                    chars.splice(start..end, wrapped.chars());
                    start = end + 2;
                } else if skip {
                    result.push(arrow(Key::ArrowRight));
                    start += 1;
                } else if let Some(close) = close {
                    result.push(Event::Text(format!("{ch}{close}")));
                    result.push(arrow(Key::ArrowLeft));
                    chars.splice(start..end, [ch, close]);
                    start += 1;
                } else {
                    result.push(event.clone());
                    chars.splice(start..end, insert.chars());
                    start += insert.chars().count();
                }
                end = start;
            }
            _ => {
                // A cursor move, deletion, undo, or click needs TextEdit's own layout
                // and state. Let it handle the rest of this input batch unchanged.
                let changes_cursor = match event {
                    Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        modifiers.command
                            || matches!(
                                key,
                                Key::ArrowLeft
                                    | Key::ArrowRight
                                    | Key::ArrowUp
                                    | Key::ArrowDown
                                    | Key::Home
                                    | Key::End
                                    | Key::PageUp
                                    | Key::PageDown
                                    | Key::Backspace
                                    | Key::Delete
                                    | Key::Enter
                                    | Key::Tab
                            )
                    }
                    Event::PointerButton { .. } | Event::Cut | Event::Ime(ImeEvent::Commit(_)) => {
                        true
                    }
                    _ => false,
                };
                if changes_cursor {
                    can_assist = false;
                }
                result.push(event.clone());
            }
        }
    }
    result
}
