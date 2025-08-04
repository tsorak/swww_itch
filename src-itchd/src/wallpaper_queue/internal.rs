use std::{borrow::Cow, collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Internal {
    pub map: Arc<Mutex<HashMap<Cow<'static, str>, Vec<Arc<String>>>>>,
}

impl Internal {
    pub fn new(into_map: HashMap<Cow<'static, str>, Vec<Arc<String>>>) -> Self {
        let map = into_map.into();

        Self {
            map: Arc::new(Mutex::new(map)),
        }
    }
}

mod test {
    #[tokio::test]
    async fn use_stack_and_heap_strings_as_k() {
        let v = super::Internal::new(
            [
                ("ALL".into(), vec![]),
                (String::from("user_playlist").into(), vec![]),
            ]
            .into(),
        );
    }
}
