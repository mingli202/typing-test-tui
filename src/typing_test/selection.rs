use std::fmt::Display;

#[derive(Debug)]
pub struct Selection<T: Display> {
    items: Vec<SelectionItem<T>>,
    selected_id: usize,
}

#[derive(Debug, PartialEq)]
pub struct SelectionItem<T: Display> {
    item: T,
    children: Vec<SelectionItem<T>>,
    id: usize,
    parent_id: Option<usize>,
    last_selected_child_id: Option<usize>,
}

impl<T: Display> SelectionItem<T> {
    /// Makes a selection item
    pub fn new(item: T) -> Self {
        SelectionItem {
            item,
            children: vec![],
            id: 0,
            parent_id: None,
            last_selected_child_id: None,
        }
    }

    /// Returns self with the given children set
    pub fn children(mut self, children: Vec<SelectionItem<T>>) -> Self {
        self.children = children;
        self
    }
}

impl<T: Display> Selection<T> {
    /// new Selection
    /// Will set the ids the item regardless they have been set or not
    pub fn new(mut items: Vec<SelectionItem<T>>) -> Self {
        let mut id = 0;

        for item in &mut items {
            id = Self::set_id(item, id, None);
        }

        Selection {
            items,
            selected_id: 0,
        }
    }

    fn set_id(item: &mut SelectionItem<T>, id: usize, parent_id: Option<usize>) -> usize {
        item.parent_id = parent_id;
        item.id = id;

        let mut last_id = id + 1;

        for child in item.children.iter_mut() {
            last_id = Self::set_id(child, last_id, Some(id));
        }

        last_id
    }

    /// Traverse the tree to select an item
    /// Will select the first item equal to the given item
    pub fn select(&mut self, item: T) {}
}

#[cfg(test)]
mod selection_test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    pub fn new_factory() {
        let items = vec![
            SelectionItem::new(0).children(vec![
                SelectionItem::new(0),
                SelectionItem::new(1).children(vec![SelectionItem::new(1)]),
                SelectionItem::new(2),
            ]),
            SelectionItem::new(1),
            SelectionItem::new(2),
        ];

        let selection = Selection::new(items);

        let expected = vec![
            SelectionItem {
                item: 0,
                id: 0,
                parent_id: None,
                last_selected_child_id: None,
                children: vec![
                    SelectionItem {
                        item: 0,
                        id: 1,
                        parent_id: Some(0),
                        last_selected_child_id: None,
                        children: vec![],
                    },
                    SelectionItem {
                        item: 1,
                        id: 2,
                        parent_id: Some(0),
                        last_selected_child_id: None,
                        children: vec![SelectionItem {
                            item: 1,
                            id: 3,
                            parent_id: Some(2),
                            last_selected_child_id: None,
                            children: vec![],
                        }],
                    },
                    SelectionItem {
                        item: 2,
                        id: 4,
                        parent_id: Some(0),
                        last_selected_child_id: None,
                        children: vec![],
                    },
                ],
            },
            SelectionItem {
                item: 1,
                id: 5,
                parent_id: None,
                last_selected_child_id: None,
                children: vec![],
            },
            SelectionItem {
                item: 2,
                id: 6,
                parent_id: None,
                last_selected_child_id: None,
                children: vec![],
            },
        ];

        assert_eq!(selection.items, expected)
    }
}
