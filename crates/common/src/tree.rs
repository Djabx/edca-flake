//! Generic parent→child tree building, shared by the TUI client's body explorer
//! and the web system page. The item type is arbitrary; callers supply how to read
//! a node's id and its parent-id chain, so the same algorithm serves both the
//! client's `BodyScan` and the web's `BodyResponse`.

use std::collections::{HashMap, HashSet};

/// A node owning its item plus its (already-ordered) children.
pub struct TreeNode<T> {
    pub item: T,
    pub children: Vec<TreeNode<T>>,
}

/// Builds a forest from a flat list. Each item attaches under the **nearest
/// ancestor present in `items`** (walking `parent_ids_of`, immediate-parent-first
/// — the immediate parent is often an unscanned barycentre/ring that isn't in the
/// set). Items with no present ancestor are roots. Roots and every child list are
/// sorted by id. Assumes an acyclic parent graph (as Elite Dangerous bodies are).
pub fn build_forest<T>(
    items: Vec<T>,
    id_of: impl Fn(&T) -> i32,
    parent_ids_of: impl Fn(&T) -> Vec<i32>,
) -> Vec<TreeNode<T>> {
    let ids: Vec<i32> = items.iter().map(&id_of).collect();
    let present: HashSet<i32> = ids.iter().copied().collect();

    // parent id → indices of its children; and which items have a present parent.
    // A node never parents itself: `id != own id` keeps a body that lists its own
    // id as a parent (malformed/untrusted data) a root instead of a child of itself.
    let mut children_of: HashMap<i32, Vec<usize>> = HashMap::new();
    let mut has_parent = vec![false; items.len()];
    for (idx, item) in items.iter().enumerate() {
        let own = ids[idx];
        if let Some(pid) = parent_ids_of(item)
            .into_iter()
            .find(|p| *p != own && present.contains(p))
        {
            children_of.entry(pid).or_default().push(idx);
            has_parent[idx] = true;
        }
    }
    for kids in children_of.values_mut() {
        kids.sort_by_key(|&i| ids[i]);
    }

    let mut roots: Vec<usize> = (0..items.len()).filter(|&i| !has_parent[i]).collect();
    roots.sort_by_key(|&i| ids[i]);

    // Move items into option slots so each is taken exactly once while building.
    let mut slots: Vec<Option<T>> = items.into_iter().map(Some).collect();

    // Returns `None` if this slot was already taken — which happens if the input
    // has duplicate ids (two nodes share a child list) or a parent cycle. Taking
    // the slot on entry means each node is built at most once and a revisit unwinds
    // gracefully instead of panicking or recursing forever on untrusted input.
    fn build<T>(
        idx: usize,
        ids: &[i32],
        children_of: &HashMap<i32, Vec<usize>>,
        slots: &mut [Option<T>],
    ) -> Option<TreeNode<T>> {
        let item = slots[idx].take()?;
        let children = children_of
            .get(&ids[idx])
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|c| build(c, ids, children_of, slots))
            .collect();
        Some(TreeNode { item, children })
    }

    roots
        .into_iter()
        .filter_map(|r| build(r, &ids, &children_of, &mut slots))
        .collect()
}

/// Depth-first flatten with ASCII tree-connector prefixes (`├ ` / `└ ` / `│ `),
/// pairing each item with the prefix that indents it. Roots get no prefix.
pub fn flatten_with_prefix<T: Clone>(forest: &[TreeNode<T>]) -> Vec<(String, T)> {
    fn walk<T: Clone>(nodes: &[TreeNode<T>], guides: &str, depth: usize, out: &mut Vec<(String, T)>) {
        let n = nodes.len();
        for (i, node) in nodes.iter().enumerate() {
            let is_last = i + 1 == n;
            let prefix = if depth == 0 {
                String::new()
            } else {
                format!("{guides}{} ", if is_last { '└' } else { '├' })
            };
            out.push((prefix, node.item.clone()));
            let child_guides = if depth == 0 {
                String::new()
            } else {
                format!("{guides}{} ", if is_last { ' ' } else { '│' })
            };
            walk(&node.children, &child_guides, depth + 1, out);
        }
    }
    let mut out = Vec::new();
    walk(forest, "", 0, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // (id, parent_id). parent 0 means "no present parent".
    fn forest(items: Vec<(i32, i32)>) -> Vec<TreeNode<(i32, i32)>> {
        build_forest(items, |t| t.0, |t| vec![t.1])
    }

    #[test]
    fn builds_simple_tree() {
        let f = forest(vec![(1, 0), (2, 1), (3, 1)]);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].item.0, 1);
        assert_eq!(f[0].children.len(), 2);
    }

    #[test]
    fn duplicate_ids_do_not_panic() {
        // Two nodes share id 1 (untrusted data); must not panic on the take().
        let f = forest(vec![(1, 0), (1, 0), (2, 1)]);
        // Both id-1 nodes survive as roots; the single child attaches to one of them.
        let total: usize = f.iter().map(count).sum();
        assert_eq!(total, 3);
    }

    #[test]
    fn self_parent_becomes_root_not_infinite_loop() {
        // A node listing its own id as parent must be a root, not recurse forever.
        let f = forest(vec![(1, 1), (2, 1)]);
        let total: usize = f.iter().map(count).sum();
        assert_eq!(total, 2);
        assert!(f.iter().any(|n| n.item.0 == 1));
    }

    #[test]
    fn parent_cycle_does_not_panic() {
        // 1 -> 2 -> 1 cycle: both have a present parent so neither is a root; the
        // forest is empty rather than a stack overflow.
        let f = forest(vec![(1, 2), (2, 1)]);
        assert!(f.iter().map(count).sum::<usize>() <= 2);
    }

    fn count(n: &TreeNode<(i32, i32)>) -> usize {
        1 + n.children.iter().map(count).sum::<usize>()
    }
}
