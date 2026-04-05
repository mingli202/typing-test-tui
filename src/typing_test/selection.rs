use std::fmt::Display;

#[derive(Debug)]
pub struct Selection<T: Display> {
    items: Vec<T>,
    selected_id: usize,
    tree: Vec<Node>,
}

#[derive(Debug)]
struct Node {
    id: usize,
    parent_id: Option<usize>,
    last_selected_child_id: Option<usize>,
    children: Vec<Node>,
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

    /// Returns the first item satisfying the predicate p.
    /// p takes the item and its id in the tree as argument
    fn find<F: Fn(&T, usize) -> bool>(&self, p: &F) -> Option<&Self> {
        if p(&self.item, self.id) {
            return Some(self);
        }

        for child in &self.children {
            let item = child.find(p);

            if item.is_some() {
                return item;
            }
        }

        None
    }
}

impl<T: Display> Selection<T> {
    /// new Selection
    /// Will set the ids the item regardless they have been set or not
    pub fn new(items: Vec<SelectionItem<T>>) -> Self {
        let mut raw_items = vec![];

        let nodes = items
            .into_iter()
            .map(|item| Self::flatten(item, &mut raw_items, None))
            .collect();

        Selection {
            items: raw_items,
            selected_id: 0,
            tree: nodes,
        }
    }

    /// Flatten given tree of item into a list of item, where each holds a reference id to its
    /// parent. Push each visitied item to items and return the item as a node in the tree
    fn flatten(item: SelectionItem<T>, items: &mut Vec<T>, parent_id: Option<usize>) -> Node {
        let id = items.len();

        items.push(item.item);

        let children = item
            .children
            .into_iter()
            .map(|child| Self::flatten(child, items, Some(id)))
            .collect();

        let node = Node {
            id,
            parent_id,
            last_selected_child_id: None,
            children,
        };

        node
    }

    /// Finds the first item equal to the given item
    pub fn find(&self, item: T) -> Option<(usize, &T)>
    where
        T: PartialEq,
    {
        self.find_with(|_, tree_item| *tree_item == item)
    }

    /// Finds the first item satisfying the predicate
    pub fn find_with<F: Fn(usize, &T) -> bool>(&self, p: F) -> Option<(usize, &T)> {
        for (i, item) in self.items.iter().enumerate() {
            if p(i, item) {
                return Some((i, item));
            }
        }

        None
    }

    /// Traverse the tree to select an item
    /// Will select the first item equal to the given item
    /// If you need a prediate instead, see select_with
    pub fn select(&mut self, item: T)
    where
        T: PartialEq,
    {
        self.select_with(|_, tree_item| *tree_item == item);
    }

    /// Traverse the tree to select the first item satisfying the predicate
    /// Predicate takes the item as argument and it's id in the tree
    /// If nothing matches, selected item is unchanged
    pub fn select_with<F: Fn(usize, &T) -> bool>(&mut self, p: F) {
        if let Some((id, _)) = self.find_with(p) {
            self.selected_id = id;
        }
    }

    /// Move the selection up a level.
    /// It will select the parent of the current selected id.
    /// If there are no parent, the selected item is unchanged
    pub fn up(&mut self) {}

    /// Move the selection down a level.
    /// It will select the previously selected child if there is.
    /// Otherwise, the first child will be selected.
    /// If there are no child, no change in the selected item
    pub fn down(&mut self) {}

    /// Select the item left of the selected item.
    /// Will look back to the end of the children.
    pub fn left(&mut self) {}

    /// Select the item right of the selected item.
    /// Will look back to the start of the children.
    pub fn right(&mut self) {}
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

    #[test]
    pub fn selection() {
        let items = vec![
            SelectionItem::new(0).children(vec![
                SelectionItem::new(0),
                SelectionItem::new(1).children(vec![SelectionItem::new(1), SelectionItem::new(5)]),
                SelectionItem::new(2),
            ]),
            SelectionItem::new(1),
            SelectionItem::new(4),
        ];

        let mut selection = Selection::new(items);

        selection.select(1);
        assert_eq!(selection.selected_id, 2);

        selection.select(5);
        assert_eq!(selection.selected_id, 4);

        selection.select_with(|_, item| *item == 4);
        assert_eq!(selection.selected_id, 7);

        selection.select(-1);
        assert_eq!(selection.selected_id, 7);
    }
}
