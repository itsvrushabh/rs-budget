//! Directed Acyclic Graph (DAG) backed by a generational slotmap arena for reactive cell dependencies.

use slotmap::{new_key_type, SlotMap};
use std::collections::{HashMap, HashSet, VecDeque};

use crate::error::DomainError;

new_key_type! {
    /// Generational arena key for dependency graph nodes.
    pub struct NodeKey;
}

/// Data associated with a cell node in the dependency graph.
#[derive(Debug, Clone)]
pub struct NodeData {
    /// Cell name identifier (e.g. "2026-11!leftover-cat1").
    pub name: String,
    /// Set of dependent node keys (cells that depend on this cell).
    pub dependents: HashSet<NodeKey>,
    /// Set of dependency node keys (cells that this cell depends on).
    pub dependencies: HashSet<NodeKey>,
}

/// Generational arena-backed directed acyclic graph tracking cell dependencies and topological recomputations.
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// Maps cell name to generational `NodeKey`.
    name_to_key: HashMap<String, NodeKey>,
    /// SlotMap generational arena storing node data.
    nodes: SlotMap<NodeKey, NodeData>,
}

impl DependencyGraph {
    /// Creates a new empty `DependencyGraph`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of nodes in the graph.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns whether the graph is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Ensures a node exists in the generational arena, returning its `NodeKey`.
    pub fn get_or_insert_node(&mut self, name: &str) -> NodeKey {
        if let Some(&key) = self.name_to_key.get(name) {
            if self.nodes.contains_key(key) {
                return key;
            }
        }

        let name_owned = name.to_string();
        let name_clone = name_owned.clone();
        let key = self.nodes.insert_with_key(|_| NodeData {
            name: name_owned,
            dependents: HashSet::new(),
            dependencies: HashSet::new(),
        });
        self.name_to_key.insert(name_clone, key);
        key
    }

    /// Returns the `NodeKey` for a cell name if it exists.
    #[must_use]
    pub fn get_node_key(&self, name: &str) -> Option<NodeKey> {
        self.name_to_key
            .get(name)
            .copied()
            .filter(|&k| self.nodes.contains_key(k))
    }

    /// Returns the cell name for a `NodeKey` if valid.
    #[must_use]
    pub fn get_node_name(&self, key: NodeKey) -> Option<&str> {
        self.nodes.get(key).map(|node| node.name.as_str())
    }

    /// Adds a directed dependency edge: `dependent` depends on `dependency`.
    ///
    /// # Errors
    /// Returns [`DomainError::CycleDetected`] if adding this edge introduces a dependency cycle.
    pub fn add_dependency(&mut self, dependency: &str, dependent: &str) -> Result<(), DomainError> {
        let u = self.get_or_insert_node(dependency);
        let v = self.get_or_insert_node(dependent);

        if u == v {
            return Err(DomainError::CycleDetected(format!(
                "self-dependency on cell '{dependency}'"
            )));
        }

        // Check if v can already reach u (which would form a cycle if u -> v is added)
        if self.can_reach(v, u) {
            return Err(DomainError::CycleDetected(format!(
                "adding dependency from '{dependency}' to '{dependent}' creates a cycle"
            )));
        }

        self.nodes[u].dependents.insert(v);
        self.nodes[v].dependencies.insert(u);
        Ok(())
    }

    /// Clears all incoming dependency edges for `dependent`.
    pub fn clear_dependencies(&mut self, dependent: &str) {
        if let Some(&v) = self.name_to_key.get(dependent) {
            if let Some(v_node) = self.nodes.get_mut(v) {
                let deps = std::mem::take(&mut v_node.dependencies);
                for u in deps {
                    if let Some(u_node) = self.nodes.get_mut(u) {
                        u_node.dependents.remove(&v);
                    }
                }
            }
        }
    }

    /// Removes a node from the graph and detaches all incoming and outgoing edges.
    pub fn remove_node(&mut self, name: &str) -> Option<NodeData> {
        let key = self.name_to_key.remove(name)?;
        let node = self.nodes.remove(key)?;

        for &u in &node.dependencies {
            if let Some(u_node) = self.nodes.get_mut(u) {
                u_node.dependents.remove(&key);
            }
        }

        for &w in &node.dependents {
            if let Some(w_node) = self.nodes.get_mut(w) {
                w_node.dependencies.remove(&key);
            }
        }

        Some(node)
    }

    /// Returns true if `start` can reach `target` through outgoing dependent edges.
    #[must_use]
    pub fn can_reach(&self, start: NodeKey, target: NodeKey) -> bool {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(start);
        visited.insert(start);

        while let Some(curr) = queue.pop_front() {
            if curr == target {
                return true;
            }

            if let Some(node) = self.nodes.get(curr) {
                for &next in &node.dependents {
                    if visited.insert(next) {
                        queue.push_back(next);
                    }
                }
            }
        }

        false
    }

    /// Topologically sorts all transitive dependents of the given source cell names.
    ///
    /// The returned list contains every cell that must be re-evaluated when the source cells change,
    /// ordered such that a cell's dependencies always precede it in the list.
    ///
    /// # Errors
    /// Returns [`DomainError::CycleDetected`] if a cycle is encountered.
    pub fn topological_sort_dependents(
        &self,
        sources: &[&str],
    ) -> Result<Vec<String>, DomainError> {
        let mut subgraph_nodes = HashSet::new();
        let mut queue = VecDeque::new();

        for &src in sources {
            if let Some(&key) = self.name_to_key.get(src) {
                if let Some(node) = self.nodes.get(key) {
                    for &dep in &node.dependents {
                        if subgraph_nodes.insert(dep) {
                            queue.push_back(dep);
                        }
                    }
                }
            }
        }

        // Collect all transitive descendants
        while let Some(curr) = queue.pop_front() {
            if let Some(node) = self.nodes.get(curr) {
                for &next in &node.dependents {
                    if subgraph_nodes.insert(next) {
                        queue.push_back(next);
                    }
                }
            }
        }

        if subgraph_nodes.is_empty() {
            return Ok(Vec::new());
        }

        // Calculate in-degrees within the induced subgraph
        let mut in_degree: HashMap<NodeKey, usize> = HashMap::new();
        for &key in &subgraph_nodes {
            if let Some(node) = self.nodes.get(key) {
                let count = node
                    .dependencies
                    .iter()
                    .filter(|dep| subgraph_nodes.contains(dep))
                    .count();
                in_degree.insert(key, count);
            }
        }

        let mut ready: VecDeque<NodeKey> = in_degree
            .iter()
            .filter_map(|(&key, &deg)| if deg == 0 { Some(key) } else { None })
            .collect();

        let mut sorted = Vec::with_capacity(subgraph_nodes.len());

        while let Some(u) = ready.pop_front() {
            if let Some(u_node) = self.nodes.get(u) {
                sorted.push(u_node.name.clone());

                for &v in &u_node.dependents {
                    if subgraph_nodes.contains(&v) {
                        if let Some(deg) = in_degree.get_mut(&v) {
                            *deg -= 1;
                            if *deg == 0 {
                                ready.push_back(v);
                            }
                        }
                    }
                }
            }
        }

        if sorted.len() != subgraph_nodes.len() {
            return Err(DomainError::CycleDetected(
                "cycle detected among dependent cells".to_string(),
            ));
        }

        Ok(sorted)
    }
}
