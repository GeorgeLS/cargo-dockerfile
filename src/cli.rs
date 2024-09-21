#[derive(Debug, clap::Parser)]
#[clap(
    author = "George Liontos",
    about = "Generate Dockerfile for your Rust project",
    version
)]
pub struct Cli {
    // This is because cargo passes the subcommand name to the program itself as the first argument
    #[clap(hide = true, required = false)]
    pub(crate) _ignore: Option<String>,
    #[clap(
        short,
        long,
        default_value = "rust:latest",
        help = "The builder image to use. Normally you would want to use rust:<tag>"
    )]
    pub builder_image: String,
    #[clap(
        short,
        long,
        help = "The runner image to use if you want to create a final runner image with your binaries in it. If not given, a runner build phase will not be generated"
    )]
    pub runner_image: Option<String>,
    #[clap(
        short,
        long,
        default_value = "/app",
        help = "The path where the binaries will be installed"
    )]
    pub app_path: String,
    #[clap(short, long, default_value_t = whoami::username(), help = "The user to create inside docker")]
    pub user: String,
    #[clap(short, long, help = "The command to set for Dockerfile CMD")]
    pub cmd: Option<String>,
    #[clap(short, long, help = "The entrypoint to set for Dockerfile ENTRYPOINT")]
    pub entrypoint: Option<String>,
}
