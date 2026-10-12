const HISTORY_LIMIT: usize = 100;

pub struct LocalHistory {
    entries: Vec<String>,
    position: usize,
}

#[allow(unused)]
impl LocalHistory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            position: 0,
        }
    }

    pub fn push(&mut self, url: String) {
        if url.is_empty() {
            return;
        }

        if self.entries.get(self.position).map(String::as_str) == Some(url.as_str()) {
            return;
        }

        self.entries.truncate(self.position + 1);
        self.entries.push(url);

        if self.entries.len() > HISTORY_LIMIT {
            let drop = self.entries.len() - HISTORY_LIMIT;
            self.entries.drain(0..drop);
        }

        self.position = self.entries.len().saturating_sub(1);
    }

    pub fn back(&mut self) -> Option<&str> {
        if self.position == 0 {
            return None;
        }
        self.position -= 1;
        self.entries.get(self.position).map(String::as_str)
    }

    pub fn forward(&mut self) -> Option<&str> {
        if self.position + 1 >= self.entries.len() {
            return None;
        }
        self.position += 1;
        self.entries.get(self.position).map(String::as_str)
    }

    pub fn can_back(&self) -> bool {
        self.position > 0
    }

    pub fn can_forward(&self) -> bool {
        self.position + 1 < self.entries.len()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.position = 0;
    }
}

impl Default for LocalHistory {
    fn default() -> Self {
        Self::new()
    }
}