use crate::block::BlockKind;

pub struct Hotbar {
    slots: Vec<BlockKind>,
    selected: usize,
}

impl Hotbar {
    pub fn new() -> Self {
        Self {
            slots: vec![
                BlockKind::Grass,
                BlockKind::Dirt,
                BlockKind::Stone,
                BlockKind::Glass,
                BlockKind::Metal,
                BlockKind::Lamp,
            ],
            selected: 0,
        }
    }

    pub fn selected(&self) -> BlockKind {
        self.slots[self.selected]
    }

    pub fn select_index(&mut self, index: usize) {
        if index < self.slots.len() {
            self.selected = index;
        }
    }

    pub fn cycle(&mut self, offset: isize) {
        if self.slots.is_empty() {
            return;
        }
        let len = self.slots.len() as isize;
        let mut index = self.selected as isize + offset;
        index = ((index % len) + len) % len;
        self.selected = index as usize;
    }

    pub fn select_block(&mut self, block: BlockKind) -> bool {
        if let Some(index) = self.slots.iter().position(|&kind| kind == block) {
            self.selected = index;
            true
        } else {
            false
        }
    }

    pub fn formatted_slots(&self) -> String {
        let mut parts = Vec::with_capacity(self.slots.len());
        for (idx, block) in self.slots.iter().enumerate() {
            let label = format!("{}:{}", idx + 1, block.display_name());
            if idx == self.selected {
                parts.push(format!(">{}<", label));
            } else {
                parts.push(format!("[{}]", label));
            }
        }
        parts.join(" ")
    }
}
