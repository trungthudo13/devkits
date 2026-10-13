//! Conservative caption matching: ambiguous/missing labels keep numbered filenames.
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
struct Bounds {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}
impl Bounds {
    fn from(node: roxmltree::Node<'_, '_>) -> Option<Self> {
        let number = |name| {
            node.attribute(name)?
                .parse::<f32>()
                .ok()
                .filter(|v| v.is_finite())
        };
        let (x, y, w, h) = (
            number("left")?,
            number("top")?,
            number("width")?,
            number("height")?,
        );
        Some(Self {
            left: x.min(x + w),
            right: x.max(x + w),
            top: y.min(y + h),
            bottom: y.max(y + h),
        })
    }
}
struct Label {
    bounds: Bounds,
    text: String,
    bold: bool,
}

pub(crate) fn captions(xml: &str) -> Result<BTreeMap<usize, Vec<Option<String>>>, String> {
    // Poppler declares an external DTD; no external resource is needed or loaded.
    let xml = xml
        .lines()
        .filter(|line| !line.trim_start().starts_with("<!DOCTYPE"))
        .collect::<Vec<_>>()
        .join("\n");
    let doc = roxmltree::Document::parse(&xml).map_err(|e| e.to_string())?;
    let mut result = BTreeMap::new();
    for page in doc.descendants().filter(|n| n.has_tag_name("page")) {
        let Some(number) = page
            .attribute("number")
            .and_then(|s| s.parse::<usize>().ok())
        else {
            continue;
        };
        let labels: Vec<_> = page
            .children()
            .filter(|n| n.has_tag_name("text"))
            .filter_map(|n| {
                let text = n
                    .descendants()
                    .filter(|n| n.is_text())
                    .filter_map(|n| n.text())
                    .collect::<String>();
                let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
                if text.chars().filter(|c| c.is_alphabetic()).count() < 2
                    || text.chars().count() > 100
                {
                    return None;
                }
                Some(Label {
                    bounds: Bounds::from(n)?,
                    text,
                    bold: n.descendants().any(|n| n.has_tag_name("b")),
                })
            })
            .collect();
        let mut names = Vec::new();
        for image in page.children().filter(|n| n.has_tag_name("image")) {
            let Some(bounds) = Bounds::from(image) else {
                names.push(None);
                continue;
            };
            let max_gap = ((bounds.bottom - bounds.top) * 0.3).clamp(20., 100.);
            let mut nearby: Vec<_> = labels
                .iter()
                .filter(|label| {
                    let b = &label.bounds;
                    let middle = (b.left + b.right) / 2.;
                    b.top >= bounds.bottom - 2.
                        && b.top - bounds.bottom <= max_gap
                        && middle >= bounds.left
                        && middle <= bounds.right
                })
                .collect();
            if nearby.iter().any(|l| l.bold) {
                nearby.retain(|l| l.bold);
            }
            nearby.sort_by(|a, b| {
                a.bounds
                    .top
                    .total_cmp(&b.bounds.top)
                    .then(a.bounds.left.total_cmp(&b.bounds.left))
            });
            let Some(first) = nearby.first() else {
                names.push(None);
                continue;
            };
            // Two independent labels at the same vertical level often describe a collage.
            if nearby
                .iter()
                .skip(1)
                .any(|l| (l.bounds.top - first.bounds.top).abs() < 5.)
            {
                names.push(None);
                continue;
            }
            let mut name = first.text.clone();
            let mut bottom = first.bounds.bottom;
            for line in nearby.iter().skip(1) {
                if line.bold == first.bold
                    && (line.bounds.left - first.bounds.left).abs() < 8.
                    && line.bounds.top >= bottom - 2.
                    && line.bounds.top - bottom <= 5.
                    && first.bold
                {
                    name.push(' ');
                    name.push_str(&line.text);
                    bottom = line.bounds.bottom;
                } else {
                    break;
                }
            }
            names.push(Some(name));
        }
        result.insert(number, names);
    }
    Ok(result)
}

pub(crate) fn slug(text: &str) -> String {
    let mut output = String::new();
    for c in text.chars().flat_map(char::to_lowercase) {
        let c = match c {
            'à' | 'á' | 'ạ' | 'ả' | 'ã' | 'â' | 'ầ' | 'ấ' | 'ậ' | 'ẩ' | 'ẫ' | 'ă' | 'ằ' | 'ắ'
            | 'ặ' | 'ẳ' | 'ẵ' => 'a',
            'è' | 'é' | 'ẹ' | 'ẻ' | 'ẽ' | 'ê' | 'ề' | 'ế' | 'ệ' | 'ể' | 'ễ' => {
                'e'
            }
            'ì' | 'í' | 'ị' | 'ỉ' | 'ĩ' => 'i',
            'ò' | 'ó' | 'ọ' | 'ỏ' | 'õ' | 'ô' | 'ồ' | 'ố' | 'ộ' | 'ổ' | 'ỗ' | 'ơ' | 'ờ' | 'ớ'
            | 'ợ' | 'ở' | 'ỡ' => 'o',
            'ù' | 'ú' | 'ụ' | 'ủ' | 'ũ' | 'ư' | 'ừ' | 'ứ' | 'ự' | 'ử' | 'ữ' => {
                'u'
            }
            'ỳ' | 'ý' | 'ỵ' | 'ỷ' | 'ỹ' => 'y',
            'đ' => 'd',
            '\u{0300}'..='\u{036f}' => continue,
            other => other,
        };
        if c.is_alphanumeric() {
            output.push(c);
        } else if !output.is_empty() && !output.ends_with('_') {
            output.push('_');
        }
        if output.chars().count() >= 80 {
            break;
        }
    }
    let result = output.trim_matches('_').to_string();
    // Avoid Windows reserved device names, even when exporting on Linux.
    if [
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
        "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ]
    .contains(&result.as_str())
    {
        format!("image_{result}")
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vietnamese_filenames_and_path_safety() {
        assert_eq!(slug("Súp bí đỏ"), "sup_bi_do");
        assert_eq!(slug("Súp cầu vồng dinh dưỡng"), "sup_cau_vong_dinh_duong");
        assert_eq!(slug("../SÚP / bí đỏ?"), "sup_bi_do");
        assert_eq!(slug("CON"), "image_con");
    }
    #[test]
    fn mirrored_images_prices_translations_and_collages() {
        let xml = r#"<pdf2xml><page number="3">
        <image top="20" left="800" width="-500" height="200"/>
        <text top="240" left="350" width="100" height="16"><b>Súp bí đỏ</b></text>
        <text top="240" left="700" width="50" height="16"><b>68.000</b></text>
        <text top="258" left="350" width="100" height="16">Pumpkin soup</text>
        <image top="300" left="300" width="500" height="200"/>
        <text top="520" left="350" width="150" height="16"><b>Súp cầu vồng dinh dưỡng</b></text>
        <image top="600" left="300" width="500" height="200"/>
        <text top="820" left="350" width="100" height="16"><b>Cuốn thính</b></text>
        <text top="820" left="600" width="100" height="16"><b>Diếp cuốn</b></text>
        </page></pdf2xml>"#;
        let names = captions(xml).unwrap();
        assert_eq!(
            names[&3],
            vec![
                Some("Súp bí đỏ".into()),
                Some("Súp cầu vồng dinh dưỡng".into()),
                None
            ]
        );
    }
}
