use crate::Cli;
use std::borrow::Cow;
use std::path::{Path, PathBuf};

use crate::constants::CARGO_TOML;

#[derive(Copy, Clone)]
enum CrateType {
    Libary,
    Binary,
}

fn make_path_mapper(
    root_path: PathBuf,
    crate_type: CrateType,
) -> impl Fn(&PathBuf) -> (Cow<str>, Cow<str>, CrateType) {
    move |path: &PathBuf| {
        let name = path.file_name().unwrap().to_string_lossy();
        let relative_path = path.strip_prefix(&root_path).unwrap().to_string_lossy();

        (name, relative_path, crate_type)
    }
}

fn generate_docker_crate_type_build(
    buffer: &mut String,
    docker_root_dir: &str,
    name: &str,
    relative_path: &str,
    crate_type: CrateType,
) {
    let (crate_type_flag, extra_rm_sources) = match crate_type {
        CrateType::Libary => ("--lib", String::new()),
        CrateType::Binary => (
            "--bin",
            format!("./target/release/deps/{}*", name.replace('-', "_")),
        ),
    };

    let prefix = if !relative_path.is_empty() {
        format!("{relative_path}")
    } else {
        ".".to_string()
    };

    buffer.push_str(&format!(
        r#"
WORKDIR {docker_root_dir}

RUN USER=root cargo new {crate_type_flag} {name}
WORKDIR ./{name}
COPY {prefix}/{CARGO_TOML} ./{CARGO_TOML}
RUN cargo build --release
RUN rm src/*.rs {extra_rm_sources}
ADD {prefix} ./
RUN cargo build --release
"#
    ))
}

pub(crate) fn get_dockerfile(root_dir: &Path) -> PathBuf {
    let mut buf = root_dir.to_path_buf();
    buf.push("Dockerfile");
    if buf.exists() {
        buf.pop();
        buf.push("cargo-dockerfile.Dockerfile")
    }

    buf
}

pub(crate) fn generate_dockerfile<'i, L, B>(root_dir: &Path, cli: &Cli, libs: L, bins: B) -> String
where
    L: Iterator<Item = &'i PathBuf>,
    B: Iterator<Item = &'i PathBuf> + Clone,
{
    let mut contents = String::new();

    contents.push_str(&format!("FROM {} as builder\n", &cli.builder_image));

    let bin_mapper = make_path_mapper(root_dir.to_path_buf(), CrateType::Binary);

    let bin_crate_as_root = bins
        .clone()
        .filter(|b| b.to_string_lossy() == root_dir.to_string_lossy())
        .next();

    let docker_root_dir = if let Some(bin) = bin_crate_as_root {
        let bin_crate_name = bin.file_name().unwrap().to_string_lossy();

        contents.push_str(&format!("RUN USER=root cargo new --bin {bin_crate_name}\n"));

        format!("/{bin_crate_name}")
    } else {
        "/".to_string()
    };

    let crate_iter = libs
        .map(make_path_mapper(root_dir.to_path_buf(), CrateType::Libary))
        .chain(
            bins.clone()
                .filter(|b| Some(*b) != bin_crate_as_root)
                .map(&bin_mapper),
        );

    for (name, rel_path, crate_type) in crate_iter {
        generate_docker_crate_type_build(
            &mut contents,
            docker_root_dir.as_ref(),
            name.as_ref(),
            rel_path.as_ref(),
            crate_type,
        );
    }

    if let Some(bin) = bin_crate_as_root {
        let bin_deps = bin.file_name().unwrap().to_string_lossy().replace('-', "_");

        contents.push_str(&format!(
            r#"
WORKDIR {docker_root_dir}

COPY ./{CARGO_TOML} ./{CARGO_TOML}
RUN cargo build --release
RUN rm src/*.rs ./target/release/deps/{bin_deps}*
ADD . ./
RUN cargo build --release
"#
        ))
    }

    if let Some(runner_image) = &cli.runner_image {
        contents.push_str(&format!("\nFROM {runner_image}"));
    }

    let copy_cmd = if cli.runner_image.is_some() {
        "COPY --from=builder"
    } else {
        "RUN cp"
    };

    contents.push_str(&format!(
        r#"
ARG APP={}
ARG APP_USER={}

RUN groupadd $APP_USER && useradd -g $APP_USER $APP_USER && mkdir -p $APP
"#,
        &cli.app_path, &cli.user
    ));

    for bin in bins {
        let (name, rel_path, _) = bin_mapper(bin);
        let prefix = rel_path
            .is_empty()
            .then(|| "".to_string())
            .unwrap_or_else(|| rel_path.to_string());

        contents.push_str(&format!(
            "{copy_cmd} {docker_root_dir}/{prefix}/target/release/{name} $APP/{name}\n"
        ));
    }

    contents.push_str(&format!(
        r#"
USER $USER
WORKDIR $APP
    "#
    ));

    if let Some(entrypoint) = &cli.entrypoint {
        let entrypoint_parts: Vec<_> = entrypoint
            .split_ascii_whitespace()
            .map(|v| format!("\"{v}\""))
            .collect();
        contents.push_str(&format!(
            r#"
ENTRYPOINT [{}]
"#,
            entrypoint_parts.join(", ")
        ));
    }

    if let Some(cmd) = &cli.cmd {
        let cmd_parts: Vec<_> = cmd
            .split_ascii_whitespace()
            .map(|v| format!("\"{v}\""))
            .collect();
        contents.push_str(&format!(
            r#"
CMD [{}]
"#,
            cmd_parts.join(", ")
        ));
    }

    contents
}
