use xtask_waw::{
    anyhow::Result,
    clap::{self},
};

#[derive(clap::Parser)]
enum Opt {
    Dist(xtask_waw::Dist),
}

fn main() -> Result<()> {
    let cmd: Opt = clap::Parser::parse();

    match cmd {
        Opt::Dist(dist) => {
            dist.release(true)
                .dist_dir_path("pkg")
                .app_name("waw-demo")
                .run_in_workspace(true)
                .run("waw-demo")?;
        }
    }

    Ok(())
}
