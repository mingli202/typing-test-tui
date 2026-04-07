use std::fmt::Display;

use ratatui::macros::{span, text};
use ratatui::style::{Color, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// A menu where menu can have their own menu items.
/// This struct handles the selection of such menu items.
/// It includes selecting up/down/left/right
#[derive(Debug)]
pub struct Selection<T> {
    root: SelectionItem<T>,
    selected_path: Vec<usize>,
}

/// A singular selection item
#[derive(Debug, PartialEq)]
pub struct SelectionItem<T> {
    /// Option because root node has technically nothing
    item: Option<T>,
    children: Vec<SelectionItem<T>>,
    last_selected_child_id: Option<usize>,
}

impl<T> Default for SelectionItem<T> {
    fn default() -> Self {
        SelectionItem {
            item: None,
            children: vec![],
            last_selected_child_id: None,
        }
    }
}

impl<T> SelectionItem<T> {
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
    /// p takes an immutable reference to self.item
    /// returns the path it took to find the item
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

impl<T> Selection<T> {
    /// new Selection
    /// Will set the ids the item regardless they have been set or not
    pub fn new(items: Vec<SelectionItem<T>>) -> Self {
        let root = SelectionItem::default().children(items);

        Selection {
            root,
            selected_path: vec![0],
        }
    }

    /// Gets immutable reference to the internal element of the currently selected item
    pub fn get_selected_item(&self) -> Option<&T> {
        self.get_selected_selection_item()
            .and_then(|item| item.item.as_ref())
    }

    /// Move the selection up a level.
    /// It will select the parent of the current selected id.
    /// If there are no parent, the selected item is unchanged
    pub fn up(&mut self) {
        if self.selected_path.len() > 1 {
            let prev_index = self.selected_path.pop().unwrap();

            if let Some(selected) = self.get_selected_selection_item_mut() {
                selected.last_selected_child_id = Some(prev_index);
            }
        }
    }

    /// Move the selection down a level.
    /// It will select the previously selected child if there is.
    /// Otherwise, the first child will be selected.
    /// If there are no child, no change in the selected item
    pub fn down(&mut self) {
        if let Some(selected) = self.get_selected_selection_item() {
            match selected.last_selected_child_id {
                Some(last_selected_child_id) => self.selected_path.push(last_selected_child_id),
                None if !selected.children.is_empty() => self.selected_path.push(0),
                _ => (),
            }
        }
    }

    /// Select the item left of the selected item.
    /// Will loop back to the end of the children.
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
    /// Will loop back to the start of the children.
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

    /// Return the selection as a renderable paragraph
    pub fn get_widget(&self) -> Paragraph<'_>
    where
        T: Display,
    {
        let mut root = &self.root;
        let mut t = text![];

        let selected_path_len = self.selected_path.len();

        for (path_index, child_index) in self.selected_path.iter().enumerate() {
            let line = Self::get_children_row(
                &root.children,
                Some(*child_index),
                Some(path_index),
                selected_path_len,
            );

            t.push_line(line);

            if let Some(child) = root.children.get(*child_index) {
                root = child;
            }
        }

        let line = Self::get_children_row(&root.children, None, None, selected_path_len);
        t.push_line(line);

        Paragraph::new(t)
    }

    /// Gets immutable reference to currently selected selection item
    fn get_selected_selection_item(&self) -> Option<&SelectionItem<T>> {
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

    /// Gets mutable reference to currently selected selection item
    fn get_selected_selection_item_mut(&mut self) -> Option<&mut SelectionItem<T>> {
        let mut selected = &mut self.root;

        for i in &self.selected_path {
            match selected.children.get_mut(*i) {
                Some(child) => selected = child,
                None => {
                    return None;
                }
            }
        }

        Some(selected)
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

    /// Gets a renderable line of selection item
    fn get_children_row(
        children: &[SelectionItem<T>],
        child_index: Option<usize>,
        path_index: Option<usize>,
        selected_path_len: usize,
    ) -> Line<'_>
    where
        T: Display,
    {
        let row = children.iter().enumerate().map(|(i, selection_item)| {
            selection_item.item.as_ref().map_or(span!(""), |item| {
                let span = Span::from(item.to_string())
                    .fg(Color::White)
                    .bg(Color::Black);

                if child_index == Some(i) {
                    if path_index == Some(selected_path_len - 1) {
                        highlight_white(span)
                    } else {
                        highlight_gray(span)
                    }
                } else {
                    span
                }
            })
        });

        let row = itertools::intersperse(row, span!(" ")).collect::<Vec<Span>>();
        Line::from(row)
    }
}

fn highlight_white(text: Span) -> Span {
    text.fg(Color::Black).bg(Color::White)
}

fn highlight_gray(text: Span) -> Span {
    text.fg(Color::Black).bg(Color::DarkGray)
}

#[cfg(test)]
mod selection_test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn selection() {
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
    fn left() {
        let items = vec![
            SelectionItem::new(0).children(vec![
                SelectionItem::new(0),
                SelectionItem::new(1).children(vec![SelectionItem::new(1), SelectionItem::new(5)]),
                SelectionItem::new(2),
            ]),
            SelectionItem::new(1),
            SelectionItem::new(4).children(vec![SelectionItem::new(6)]),
        ];

        let mut selection = Selection::new(items);

        selection.select(1);

        selection.left();
        assert_eq!(selection.selected_path, vec![0, 0]);

        selection.left();
        assert_eq!(selection.selected_path, vec![0, 2]);

        selection.left();
        assert_eq!(selection.selected_path, vec![0, 1]);

        selection.select(6);
        selection.left();
        assert_eq!(selection.selected_path, vec![2, 0]);
    }

    #[test]
    fn right() {
        let items = vec![
            SelectionItem::new(0).children(vec![
                SelectionItem::new(0),
                SelectionItem::new(1).children(vec![SelectionItem::new(1), SelectionItem::new(5)]),
                SelectionItem::new(2),
            ]),
            SelectionItem::new(1),
            SelectionItem::new(4).children(vec![SelectionItem::new(6)]),
        ];

        let mut selection = Selection::new(items);

        selection.select(1);

        selection.right();
        assert_eq!(selection.selected_path, vec![0, 2]);

        selection.right();
        assert_eq!(selection.selected_path, vec![0, 0]);

        selection.right();
        assert_eq!(selection.selected_path, vec![0, 1]);

        selection.select(6);
        selection.right();
        assert_eq!(selection.selected_path, vec![2, 0]);
    }

    #[test]
    fn up() {
        let items = vec![
            SelectionItem::new(0).children(vec![
                SelectionItem::new(0),
                SelectionItem::new(1).children(vec![SelectionItem::new(1), SelectionItem::new(5)]),
                SelectionItem::new(2),
            ]),
            SelectionItem::new(1),
            SelectionItem::new(4).children(vec![SelectionItem::new(6)]),
        ];

        let mut selection = Selection::new(items);

        selection.select(5);

        selection.up();
        assert_eq!(selection.selected_path, vec![0, 1]);

        selection.up();
        assert_eq!(selection.selected_path, vec![0]);

        selection.up();
        assert_eq!(selection.selected_path, vec![0]);
    }

    #[test]
    fn down() {
        let items = vec![
            SelectionItem::new(0).children(vec![
                SelectionItem::new(0),
                SelectionItem::new(1).children(vec![SelectionItem::new(1), SelectionItem::new(5)]),
                SelectionItem::new(2),
            ]),
            SelectionItem::new(1),
            SelectionItem::new(4).children(vec![SelectionItem::new(6)]),
        ];

        let mut selection = Selection::new(items);

        selection.select(0);

        selection.down();
        assert_eq!(selection.selected_path, vec![0, 0]);

        selection.down();
        assert_eq!(selection.selected_path, vec![0, 0]);

        selection.select(1);
        selection.down();
        assert_eq!(selection.selected_path, vec![0, 1, 0]);
    }

    #[test]
    fn up_down() {
        let items = vec![
            SelectionItem::new(0).children(vec![
                SelectionItem::new(0),
                SelectionItem::new(1).children(vec![SelectionItem::new(1), SelectionItem::new(5)]),
                SelectionItem::new(2),
            ]),
            SelectionItem::new(1),
            SelectionItem::new(4).children(vec![SelectionItem::new(6)]),
        ];

        let mut selection = Selection::new(items);

        selection.select(1);

        selection.up();
        assert_eq!(selection.selected_path, vec![0]);

        selection.down();
        assert_eq!(selection.selected_path, vec![0, 1]);
    }
}
