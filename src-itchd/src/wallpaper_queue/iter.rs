use super::Queue;

pub trait BackgroundIterable {
    fn get_index_of_bg<S: AsRef<str>>(&self, bg: S) -> Option<usize>;
    fn get_bg<S: AsRef<str>>(&self, bg: S) -> Option<&String>;
    fn to_vec(&self) -> Vec<String>;
    fn remove(&mut self, index: usize) -> String;
    fn insert(&mut self, index: usize, item: String);
    fn get_by_index(&self, bg: usize) -> Option<&String>;
}

// impl<I: Iterator<Item = String> + Clone> BackgroundIterable for I {
//     fn get_bg<S: AsRef<str>>(&mut self, bg: S) -> Option<String> {
//         self.find(|v| v.as_str().ends_with(bg.as_ref()))
//     }
//     fn get_index_of_bg<S: AsRef<str>>(&mut self, bg: S) -> Option<usize> {
//         self.position(|v| v.as_str().ends_with(bg.as_ref()))
//     }
//     fn collect<B: FromIterator<String>>(&self) -> B {
//         Iterator::collect(self.clone())
//     }
// }

impl BackgroundIterable for Queue {
    fn get_bg<S: AsRef<str>>(&self, bg: S) -> Option<&String> {
        self.v.iter().find(|v| v.as_str().ends_with(bg.as_ref()))
    }
    fn get_index_of_bg<S: AsRef<str>>(&self, bg: S) -> Option<usize> {
        self.v
            .iter()
            .position(|v| v.as_str().ends_with(bg.as_ref()))
    }
    fn to_vec(&self) -> Vec<String> {
        self.v.clone()
    }
    fn remove(&mut self, index: usize) -> String {
        self.v.remove(index)
    }
    fn insert(&mut self, index: usize, item: String) {
        self.v.insert(index, item)
    }
    fn get_by_index(&self, bg: usize) -> Option<&String> {
        self.v.get(bg)
    }
}
