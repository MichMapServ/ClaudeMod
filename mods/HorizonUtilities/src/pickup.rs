//! The pickup notifications' state: pure, driven by real pickup events and
//! the elapsed time of the `frame` hook (never by the frame rate).
use crate::nutrition::display_name;

const FADE_IN: f32 = 0.2;
const FADE_OUT: f32 = 0.4;

struct Entry {
    item: String,
    count: u32,
    age: f32,
    label: String,
    label_w: f32,
}

/// One visible notification, newest first.
pub struct Row<'a> {
    pub item: &'a str,
    pub label: &'a str,
    pub label_w: f32,
    /// 0 (not there) to 1 (fully there), eased: drives the slide.
    pub visible: f32,
}

pub struct Notifier {
    entries: Vec<Entry>,
    max: usize,
    lifetime: f32,
}

fn label(item: &str, count: u32) -> String {
    format!("+{count} {}", display_name(item))
}

fn visibility(age: f32, lifetime: f32) -> f32 {
    let t = if age < FADE_IN {
        age / FADE_IN
    } else if age > lifetime - FADE_OUT {
        (lifetime - age) / FADE_OUT
    } else {
        1.0
    };
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Notifier {
    pub fn new(max: usize, lifetime: f32) -> Self {
        let mut n = Notifier {
            entries: Vec::new(),
            max: 1,
            lifetime: 3.0,
        };
        n.configure(max, lifetime);
        n
    }

    pub fn configure(&mut self, max: usize, lifetime: f32) {
        self.max = max.max(1);
        self.lifetime = if lifetime.is_finite() {
            lifetime.max(FADE_IN + FADE_OUT + 0.1)
        } else {
            3.0
        };
        self.trim();
    }

    fn trim(&mut self) {
        while self.entries.len() > self.max {
            self.entries.remove(0);
        }
    }

    /// A real pickup. The same item as a live notification adds to it and
    /// keeps it on screen; at most `max` are shown, the oldest leaving first.
    pub fn push(&mut self, item: &str, count: u32, width_of: &dyn Fn(&str) -> f32) {
        if count == 0 {
            return;
        }
        if let Some(e) = self.entries.iter_mut().find(|e| e.item == item) {
            e.count = e.count.saturating_add(count);
            e.age = e.age.min(FADE_IN);
            e.label = label(item, e.count);
            e.label_w = width_of(&e.label);
            return;
        }
        let text = label(item, count);
        let label_w = width_of(&text);
        self.entries.push(Entry {
            item: item.to_string(),
            count,
            age: 0.0,
            label: text,
            label_w,
        });
        self.trim();
    }

    /// `seconds` since the last call; a bad value changes nothing.
    pub fn advance(&mut self, seconds: f32) {
        if !seconds.is_finite() || seconds <= 0.0 {
            return;
        }
        let life = self.lifetime;
        for e in &mut self.entries {
            e.age += seconds;
        }
        self.entries.retain(|e| e.age < life);
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn rows(&self) -> Vec<Row<'_>> {
        self.entries
            .iter()
            .rev()
            .map(|e| Row {
                item: &e.item,
                label: &e.label,
                label_w: e.label_w,
                visible: visibility(e.age, self.lifetime),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn width(text: &str) -> f32 {
        text.chars().count() as f32 * 6.0
    }

    #[test]
    fn identical_items_merge_and_keep_their_slide() {
        let mut n = Notifier::new(5, 3.0);
        n.push("minecraft:stone", 3, &width);
        n.advance(1.0);
        n.push("minecraft:stone", 2, &width);
        let rows = n.rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "+5 Stone");
        assert_eq!(rows[0].label_w, width("+5 Stone"));
    }

    #[test]
    fn newest_first_and_capped() {
        let mut n = Notifier::new(2, 3.0);
        for item in ["minecraft:dirt", "minecraft:stone", "minecraft:sand"] {
            n.push(item, 1, &width);
        }
        let rows = n.rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].item, "minecraft:sand");
        assert_eq!(rows[1].item, "minecraft:stone");
    }

    #[test]
    fn entries_expire() {
        let mut n = Notifier::new(5, 2.0);
        n.push("minecraft:dirt", 1, &width);
        n.advance(1.9);
        assert!(!n.is_empty());
        n.advance(0.2);
        assert!(n.is_empty());
    }

    #[test]
    fn slide_in_hold_slide_out() {
        let mut n = Notifier::new(5, 3.0);
        n.push("minecraft:dirt", 1, &width);
        assert_eq!(n.rows()[0].visible, 0.0);
        n.advance(0.1);
        let mid = n.rows()[0].visible;
        assert!(mid > 0.0 && mid < 1.0);
        n.advance(1.0);
        assert_eq!(n.rows()[0].visible, 1.0);
        n.advance(1.8);
        let fading = n.rows()[0].visible;
        assert!(fading < 1.0 && fading >= 0.0);
    }

    #[test]
    fn zero_counts_and_bad_times_do_nothing() {
        let mut n = Notifier::new(5, 3.0);
        n.push("minecraft:dirt", 0, &width);
        assert!(n.is_empty());
        n.push("minecraft:dirt", 1, &width);
        n.advance(f32::NAN);
        n.advance(-1.0);
        assert_eq!(n.rows()[0].visible, 0.0);
    }

    #[test]
    fn configure_trims_and_bounds_the_lifetime() {
        let mut n = Notifier::new(5, 3.0);
        for item in ["minecraft:a", "minecraft:b", "minecraft:c"] {
            n.push(item, 1, &width);
        }
        n.configure(1, 0.0);
        assert_eq!(n.rows().len(), 1);
        n.advance(0.5);
        assert!(!n.is_empty());
    }

    #[test]
    fn counts_saturate() {
        let mut n = Notifier::new(5, 3.0);
        n.push("minecraft:dirt", u32::MAX, &width);
        n.push("minecraft:dirt", 5, &width);
        assert!(n.rows()[0].label.starts_with(&format!("+{}", u32::MAX)));
    }
}
