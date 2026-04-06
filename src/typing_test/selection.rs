use std::fmt::Display;

#[derive(Debug)]
pub struct Selection<T: Display> {
    root: SelectionItem<T>,
    selected_path: Vec<usize>,
}

#[derive(Debug, PartialEq)]
pub struct SelectionItem<T: Display> {
    item: Option<T>,
    children: Vec<SelectionItem<T>>,
    last_selected_child_id: Option<usize>,
}

impl<T: Display> Default for SelectionItem<T> {
    fn default() -> Self {
        SelectionItem {
            item: None,
            children: vec![],
            last_selected_child_id: None,
        }
    }
}

impl<T: Display> SelectionItem<T> {
    /// Makes a selection item
    pub fn new(item: T) -> Self {
        SelectionItem {
            item: Some(item),
            children: vec![],
            last_selected_child_id: None,
        }
    }

    /// Returns self with the given children set
    pub fn children(mut self, children: Vec<SelectionItem<T>>) -> Self {
        self.children = children;
        self
    }

    /// Returns the first item satisfying the predicate p.
    /// path is path accumulator
    /// p takes the item
    fn find_with<F: Fn(&T) -> bool>(&self, p: &F) -> Option<Vec<usize>> {
        if let Some(item) = &self.item
            && p(item)
        {
            return Some(vec![]);
        }

        for (i, child) in self.children.iter().enumerate() {
            if let Some(mut path) = child.find_with(p) {
                path.push(i);
                return Some(path);
            }
        }

        None
    }
}

impl<T: Display> Selection<T> {
    /// new Selection
    /// Will set the ids the item regardless they have been set or not
    pub fn new(items: Vec<SelectionItem<T>>) -> Self {
        let root = SelectionItem::default().children(items);

        Selection {
            root,
            selected_path: vec![0],
        }
    }

    /// Gets the currently selected item
    pub fn get_selected_item(&self) -> Option<&SelectionItem<T>> {
        let mut selected = &self.root;

        for i in &self.selected_path {
            match selected.children.get(*i) {
                Some(child) => selected = child,
                None => {
                    return None;
                }
            }
        }

        Some(selected)
    }

    /// Move the selection up a level.
    /// It will select the parent of the current selected id.
    /// If there are no parent, the selected item is unchanged
    pub fn up(&mut self) {
        if self.selected_path.len() > 1 {
            self.selected_path.pop();
        }
    }

    /// Move the selection down a level.
    /// It will select the previously selected child if there is.
    /// Otherwise, the first child will be selected.
    /// If there are no child, no change in the selected item
    pub fn down(&mut self) {
        if let Some(selected) = self.get_selected_item() {
            match selected.last_selected_child_id {
                Some(last_selected_child_id) => self.selected_path.push(last_selected_child_id),
                None if !selected.children.is_empty() => self.selected_path.push(0),
                _ => (),
            }
        }
    }

    /// Select the item left of the selected item.
    /// Will look back to the end of the children.
    pub fn left(&mut self) {
        if let Some(parent) = self.parent_of_selected()
            && !parent.children.is_empty()
        {
            let children_len = parent.children.len();

            if let Some(child_index) = self.selected_path.pop() {
                let is_first_child = child_index == 0;

                let prev_index = if is_first_child {
                    children_len - 1
                } else {
                    child_index - 1
                };

                self.selected_path.push(prev_index);
            }
        }
    }

    /// Select the item right of the selected item.
    /// Will look back to the start of the children.
    pub fn right(&mut self) {
        if let Some(parent) = self.parent_of_selected()
            && !parent.children.is_empty()
        {
            let children_len = parent.children.len();

            if let Some(child_index) = self.selected_path.pop() {
                let is_last_child = child_index == children_len - 1;

                let next_index = if is_last_child { 0 } else { child_index + 1 };

                self.selected_path.push(next_index);
            }
        }
    }

    /// Traverse the tree to select an item
    /// Will select the first item equal to the given item
    /// If you need a prediate instead, see select_with
    pub fn select(&mut self, item: T)
    where
        T: PartialEq,
    {
        self.select_with(|tree_item| *tree_item == item);
    }

    /// Traverse the tree to select the first item satisfying the predicate
    /// Predicate takes the item as argument and it's id in the tree
    /// If nothing matches, selected item is unchanged
    pub fn select_with<F: Fn(&T) -> bool>(&mut self, p: F) {
        if let Some(mut selected_path) = self.root.find_with(&p) {
            selected_path.reverse();
            self.selected_path = selected_path;
        }
    }

    /// Gets the parent of the currently selected item
    fn parent_of_selected(&self) -> Option<&SelectionItem<T>> {
        let len = self.selected_path.len();

        if len == 0 {
            return None;
        }

        let mut parent = &self.root;

        for i in &self.selected_path[..len - 1] {
            match parent.children.get(*i) {
                Some(child) => parent = child,
                None => {
                    return None;
                }
            }
        }

        Some(parent)
    }
}

#[cfg(test)]
mod selection_test {
    use super::*;
    use pretty_assertions::assert_eq;

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
        assert_eq!(selection.selected_path, vec![0, 1]);

        selection.select(5);
        assert_eq!(selection.selected_path, vec![0, 1, 1]);

        selection.select_with(|item| *item == 4);
        assert_eq!(selection.selected_path, vec![2]);

        selection.select(-1);
        assert_eq!(selection.selected_path, vec![2]);
    }

    #[test]
    fn left() {}
}
