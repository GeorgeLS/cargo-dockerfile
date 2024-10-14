mod cli;
mod constants;
mod dockerfile;
mod predicates;

use crate::cli::Cli;
use crate::constants::{CARGO_TOML, LIB_RS, MAIN_RS};
use crate::dockerfile::generate_dockerfile;
use crate::predicates::{entry_predicate, is_hidden};
use cargo_toml::Manifest;
use clap::Parser;
use dockerfile::get_dockerfile;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

struct Graph<'p> {
    libs: &'p [PathBuf],
    // (from, to) This denotes dependency from a library at index `from` to an index `to`
    // The indices are indices in the original `libs` array
    pub edges: Vec<(usize, usize)>,
    pub path_to_index_map: HashMap<&'p PathBuf, usize>,
}

impl<'p> Graph<'p> {
    pub fn from_libs(libs: &'p [PathBuf]) -> anyhow::Result<Self> {
        // Since we don't have circular dependencies between the libraries,
        // our dependency graph is a DAG. The maximum number of edges in a DAG
        // is the sum 1 + 2 + ... + n - 1 where n is the number of nodes.
        // This is the sum of first n - 1 natural numbers which can be computed
        // directly from the formula below (check discrete maths :D)
        let max_edge_num: usize = if libs.is_empty() {
            0
        } else {
            libs.len() * (libs.len() - 1) / 2
        };
        let mut edges: Vec<(usize, usize)> = Vec::with_capacity(max_edge_num);
        let path_to_index_map: HashMap<_, _> =
            libs.iter().enumerate().map(|(i, e)| (e, i)).collect();
        for (i, lib) in libs.iter().enumerate() {
            let cargo_toml_path: String = get_cargo_toml_path(lib);
            let manifest: Manifest = Manifest::from_path(cargo_toml_path)?;
            for dep in manifest
                .dependencies
                .values()
                .filter_map(|dep| dep.detail())
                .filter_map(|detail| detail.path.as_ref())
                .filter_map(|path| {
                    let mut full_path: PathBuf = lib.clone();
                    full_path.push(path);
                    full_path.canonicalize().ok()
                })
            {
                if let Some(index) = path_to_index_map.get(&dep) {
                    edges.push((i, *index));
                }
            }
        }
        Ok(Self {
            libs,
            edges,
            path_to_index_map,
        })
    }

    pub fn get_topologically_sorted_libs(&self) -> Vec<&'p PathBuf> {
        let mut dependency_histogram: Vec<u32> = vec![0; self.libs.len()];
        for (_, i) in &self.edges {
            dependency_histogram[*i] += 1u32;
        }
        let mut topologically_sorted_paths: Vec<&PathBuf> = Vec::with_capacity(self.libs.len());
        let mut stack: Vec<_> = dependency_histogram
            .iter()
            .enumerate()
            .filter(|&(_, h)| *h == 0)
            .map(|(i, _)| &self.libs[i])
            .collect();
        while let Some(path) = stack.pop() {
            topologically_sorted_paths.push(path);
            let path_index: &usize = self.path_to_index_map.get(path).unwrap();
            for dep_index in self
                .edges
                .iter()
                .filter_map(|(from, to)| (from == path_index).then_some(*to))
            {
                dependency_histogram[dep_index] -= 1;
                if dependency_histogram[dep_index] == 0 {
                    stack.push(&self.libs[dep_index]);
                }
            }
        }
        topologically_sorted_paths
    }
}

fn get_cargo_toml_path(buf: &Path) -> String {
    let mut path_buf: PathBuf = buf.to_path_buf();
    path_buf.push(CARGO_TOML);
    path_buf.to_string_lossy().to_string()
}

fn entry_root_directory(entry: &DirEntry) -> PathBuf {
    let mut path: PathBuf = entry.path().to_path_buf();
    path.pop();
    path.pop();
    path
}

fn get_crate_libs_and_bins(root_dir: &PathBuf) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let (libs, bins): (Vec<_>, Vec<_>) = WalkDir::new(root_dir)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .filter_map(std::result::Result::ok)
        .filter(entry_predicate)
        .partition(|e| e.file_name().to_str().is_some_and(|s| s == LIB_RS));
    let libs: Vec<_> = libs.iter().map(entry_root_directory).collect();
    let bins: Vec<_> = bins.iter().map(entry_root_directory).collect();
    (libs, bins)
}

fn main() -> anyhow::Result<()> {
    let cli: Cli = Cli::parse();
    let root_dir: PathBuf = std::env::current_dir()?;
    let (libs, bins) = get_crate_libs_and_bins(&root_dir);
    let graph: Graph<'_> = Graph::from_libs(&libs)?;
    let sorted_libs: Vec<&PathBuf> = graph.get_topologically_sorted_libs();
    let contents: String = generate_dockerfile(
        &root_dir,
        &cli,
        sorted_libs.iter().rev().copied(),
        bins.iter(),
    );
    let dockerfile: PathBuf = get_dockerfile(&root_dir);
    std::fs::write(dockerfile, contents).expect("couldn't write dockerfile");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_should_generate_for_basic_example() {
        let cli = crate::cli::Cli {
            _ignore: None,
            builder_image: "rust:latest".into(),
            runner_image: None,
            app_path: "/code".into(),
            user: "root".into(),
            cmd: None,
            entrypoint: None,
        };
        let root_dir = std::env::current_dir()
            .unwrap()
            .join("examples")
            .join("basic");
        let (libs, bins) = crate::get_crate_libs_and_bins(&root_dir);
        let graph = crate::Graph::from_libs(&libs).unwrap();
        let sorted_libs = graph.get_topologically_sorted_libs();
        let content: String = super::generate_dockerfile(
            &root_dir,
            &cli,
            sorted_libs.iter().rev().copied(),
            bins.iter(),
        );
        assert_eq!(content, include_str!("../examples/basic.Dockerfile"));
    }

    #[test]
    fn it_should_generate_for_basic_example_with_multistage() {
        let cli = crate::cli::Cli {
            _ignore: None,
            builder_image: "rust:latest".into(),
            runner_image: Some("rust:latest".into()),
            app_path: "/code".into(),
            user: "root".into(),
            cmd: None,
            entrypoint: None,
        };
        let root_dir = std::env::current_dir()
            .unwrap()
            .join("examples")
            .join("basic");
        let (libs, bins) = crate::get_crate_libs_and_bins(&root_dir);
        let graph = crate::Graph::from_libs(&libs).unwrap();
        let sorted_libs = graph.get_topologically_sorted_libs();
        let content: String = super::generate_dockerfile(
            &root_dir,
            &cli,
            sorted_libs.iter().rev().copied(),
            bins.iter(),
        );
        assert_eq!(
            content,
            include_str!("../examples/basic-multistage.Dockerfile")
        );
    }
}
