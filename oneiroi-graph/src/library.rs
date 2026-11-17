//! Several graphs which can use each other.

use std::{
    collections::{BTreeMap, HashMap},
    fmt,
};

use oneiroi_core::{
    node::NodeRegistry,
    program::{ProgramId, ProgramSet},
};

use crate::{Context, Graph, LowerError};

#[derive(Debug, Clone, PartialEq)]
pub enum LibraryError {
    /// The graph uses itself, directly or through other graphs.
    Cycle(String),
    Lower { graph: String, error: LowerError },
}

impl fmt::Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cycle(graph) => write!(f, "the graph `{graph}` uses itself"),
            Self::Lower { graph, error } => write!(f, "graph `{graph}`, {error}"),
        }
    }
}

impl std::error::Error for LibraryError {}

/// Graphs by their name.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Library {
    graphs: BTreeMap<String, Graph>,
}

impl Library {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a graph under its name, replacing the one that had it before.
    pub fn insert(&mut self, graph: Graph) -> Option<Graph> {
        self.graphs.insert(graph.name.clone(), graph)
    }

    pub fn remove(&mut self, name: &str) -> Option<Graph> {
        self.graphs.remove(name)
    }

    pub fn get(&self, name: &str) -> Option<&Graph> {
        self.graphs.get(name)
    }

    /// A graph to edit. Its name must stay the one it was added under.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Graph> {
        self.graphs.get_mut(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Graph> {
        self.graphs.values()
    }

    pub fn len(&self) -> usize {
        self.graphs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.graphs.is_empty()
    }

    /// Whether the graph `name` uses the graph `other`, however indirect.
    /// A graph also counts as using itself.
    pub fn uses(&self, name: &str, other: &str) -> bool {
        let mut seen = Vec::new();
        let mut open = vec![name];
        while let Some(name) = open.pop() {
            if name == other {
                return true;
            }
            if !seen.contains(&name) {
                seen.push(name);
                open.extend(self.graphs.get(name).into_iter().flat_map(Graph::dependencies));
            }
        }
        false
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a library is plain data")
    }

    /// Reads a library back. A file which holds a single graph is read as
    /// a library of that one graph.
    pub fn from_json(json: &str) -> Result<Self, crate::FormatError> {
        if let Ok(graph) = Graph::from_json(json) {
            let mut library = Self::new();
            library.insert(graph);
            return Ok(library);
        }
        let library: Self = serde_json::from_str(json).map_err(crate::FormatError::Syntax)?;
        for graph in library.iter() {
            // Checks the version of every graph in there.
            Graph::from_json(&graph.to_json())?;
        }
        Ok(library)
    }

    /// Like [`Library::lower_into`], but a graph which is broken does not
    /// stop the others: every graph that can be lowered is, and what is
    /// wrong with the rest is returned next to them.
    pub fn lower_all(
        &self,
        registry: &NodeRegistry,
        programs: &mut ProgramSet,
    ) -> (HashMap<String, ProgramId>, Vec<LibraryError>) {
        let mut lowered = HashMap::new();
        let mut errors = Vec::new();
        for name in self.graphs.keys() {
            let mut visiting = Vec::new();
            if let Err(error) = self.lower_graph(name, registry, programs, &mut lowered, &mut visiting)
                && !errors.contains(&error)
            {
                errors.push(error);
            }
        }
        (lowered, errors)
    }

    /// Lowers every graph into `programs`, each one after the graphs it
    /// uses, and returns which program belongs to which graph.
    ///
    /// A graph whose program is in `programs` already gets that one again.
    /// Lowering after an edit therefore only adds programs for the graphs
    /// which do something else now, and moving a node around adds none.
    ///
    /// The programs of before the edit stay in `programs`. Drop them with
    /// [`ProgramSet::retain`], keeping what this returns and whatever
    /// else still has instances.
    pub fn lower_into(
        &self,
        registry: &NodeRegistry,
        programs: &mut ProgramSet,
    ) -> Result<HashMap<String, ProgramId>, LibraryError> {
        let mut lowered = HashMap::new();
        let mut visiting = Vec::new();
        for name in self.graphs.keys() {
            self.lower_graph(name, registry, programs, &mut lowered, &mut visiting)?;
        }
        Ok(lowered)
    }

    fn lower_graph(
        &self,
        name: &str,
        registry: &NodeRegistry,
        programs: &mut ProgramSet,
        lowered: &mut HashMap<String, ProgramId>,
        visiting: &mut Vec<String>,
    ) -> Result<(), LibraryError> {
        if lowered.contains_key(name) {
            return Ok(());
        }
        // A graph which is not in here is reported where it is used.
        let Some(graph) = self.graphs.get(name) else {
            return Ok(());
        };
        if visiting.iter().any(|open| open == name) {
            return Err(LibraryError::Cycle(name.into()));
        }

        visiting.push(name.into());
        for dependency in graph.dependencies() {
            self.lower_graph(dependency, registry, programs, lowered, visiting)?;
        }
        visiting.pop();

        let context = Context {
            registry,
            programs,
            graphs: lowered,
        };
        let program = graph.lower(context).map_err(|error| LibraryError::Lower {
            graph: name.into(),
            error,
        })?;
        let id = match programs.find(program.hash) {
            Some(id) => id,
            None => programs.add(program),
        };
        lowered.insert(name.into(), id);
        Ok(())
    }
}
