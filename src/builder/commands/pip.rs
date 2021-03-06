use std::fs::File;
use std::io::{BufReader, BufRead};
use std::path::Path;

use super::super::context::{Context};
use super::super::packages;
use super::generic::{run_command_at_env, capture_command};
use builder::distrib::Distribution;
use config::builders::PipSettings;
use file_util::create_dir;


impl Default for PipSettings {
    fn default() -> PipSettings {
        PipSettings {
            find_links: Vec::new(),
            index_urls: Vec::new(),
            trusted_hosts: Vec::new(),
            dependencies: false,
            cache_wheels: true,
        }
    }
}


pub fn scan_features(ver: u8, pkgs: &Vec<String>) -> Vec<packages::Package> {
    let mut res = vec!();
    res.push(packages::BuildEssential);
    if ver == 2 {
        res.push(packages::Python2);
        res.push(packages::Python2Dev);
        res.push(packages::PipPy2);
    } else {
        res.push(packages::Python3);
        res.push(packages::Python3Dev);
        res.push(packages::PipPy3);
    }
    for name in pkgs.iter() {
        if name[..].starts_with("git+https") {
            res.push(packages::Git);
            res.push(packages::Https);
        } else if name[..].starts_with("git+") {
            res.push(packages::Git);
        } else if name[..].starts_with("hg+https") {
            res.push(packages::Mercurial);
            res.push(packages::Https);
        } else if name[..].starts_with("hg+") {
            res.push(packages::Mercurial);
        }
    }
    return res;
}

fn pip_args(ctx: &mut Context, ver: u8) -> Vec<String> {
    let mut args = vec!(
        (if ver == 2 { "python2" } else { "python3" }).to_string(),
        "-m".to_string(), "pip".to_string(),
        "install".to_string(),
        "--ignore-installed".to_string(),
        );
    if ctx.pip_settings.index_urls.len() > 0 {
        let mut indexes = ctx.pip_settings.index_urls.iter();
        if let Some(ref lnk) = indexes.next() {
            args.push(format!("--index-url={}", lnk));
            for lnk in indexes {
                args.push(format!("--extra-index-url={}", lnk));
            }
        }
    }
    ctx.pip_settings.trusted_hosts.iter().map(|h| {
        args.push("--trusted-host".to_string());
        args.push(h.to_string());
    }).last();
    if !ctx.pip_settings.dependencies {
        args.push("--no-deps".to_string());
    }
    for lnk in ctx.pip_settings.find_links.iter() {
        args.push(format!("--find-links={}", lnk));
    }
    return args;
}

pub fn pip_install(distro: &mut Box<Distribution>, ctx: &mut Context,
    ver: u8, pkgs: &Vec<String>)
    -> Result<(), String>
{
    try!(packages::ensure_packages(distro, ctx,
        &scan_features(ver, pkgs)[0..]));
    let mut pip_cli = pip_args(ctx, ver);
    pip_cli.extend(pkgs.clone().into_iter());
    run_command_at_env(ctx, &pip_cli, &Path::new("/work"), &[
        ("PYTHONPATH", "/tmp/non-existent:/tmp/pip-install")])
}

pub fn pip_requirements(distro: &mut Box<Distribution>, ctx: &mut Context,
    ver: u8, reqtxt: &Path)
    -> Result<(), String>
{
    let f = try!(File::open(&Path::new("/work").join(reqtxt))
        .map_err(|e| format!("Can't open requirements file: {}", e)));
    let f = BufReader::new(f);
    let mut names = vec!();
    for line in f.lines() {
        let line = try!(line
                .map_err(|e| format!("Error reading requirements: {}", e)));
        let chunk = line[..].trim();
        // Ignore empty lines and comments
        if chunk.len() == 0 || chunk.starts_with("#") {
            continue;
        }
        names.push(chunk.to_string());
    }

    try!(packages::ensure_packages(distro, ctx,
        &scan_features(ver, &names)[0..]));
    let mut pip_cli = pip_args(ctx, ver);
    pip_cli.push("--requirement".to_string());
    pip_cli.push(reqtxt.display().to_string()); // TODO(tailhook) fix conversion
    run_command_at_env(ctx, &pip_cli, &Path::new("/work"), &[
        ("PYTHONPATH", "/tmp/non-existent:/tmp/pip-install")])
}

pub fn configure(ctx: &mut Context) -> Result<(), String> {
    let cache_root = Path::new("/vagga/root/tmp/pip-cache");
    try_msg!(create_dir(&cache_root, true),
         "Error creating cache dir {d:?}: {err}", d=cache_root);

    try!(ctx.add_cache_dir(Path::new("/tmp/pip-cache/http"),
                           "pip-cache-http".to_string()));

    if ctx.pip_settings.cache_wheels {
        let cache_dir = format!("pip-cache-wheels-{}", ctx.binary_ident);
        try!(ctx.add_cache_dir(Path::new("/tmp/pip-cache/wheels"), cache_dir));
    } // else just write files in tmp

    ctx.environ.insert("PIP_CACHE_DIR".to_string(),
                       "/tmp/pip-cache".to_string());
    Ok(())
}

pub fn freeze(ctx: &mut Context) -> Result<(), String> {
    use std::fs::File;  // TODO(tailhook) migrate whole module
    use std::io::Write;  // TODO(tailhook) migrate whole module
    if ctx.featured_packages.contains(&packages::PipPy2) {
        try!(capture_command(ctx, &[
                "python2".to_string(),
                "-m".to_string(),
                "pip".to_string(),
                "freeze".to_string(),
            ], &[("PYTHONPATH", "/tmp/non-existent:/tmp/pip-install")])
            .and_then(|out| {
                File::create("/vagga/container/pip2-freeze.txt")
                .and_then(|mut f| f.write_all(&out))
                .map_err(|e| format!("Error dumping package list: {}", e))
            }));
    }
    if ctx.featured_packages.contains(&packages::PipPy3) {
        try!(capture_command(ctx, &[
                "python3".to_string(),
                "-m".to_string(),
                "pip".to_string(),
                "freeze".to_string(),
            ], &[("PYTHONPATH", "/tmp/non-existent:/tmp/pip-install")])
            .and_then(|out| {
                File::create("/vagga/container/pip3-freeze.txt")
                .and_then(|mut f| f.write_all(&out))
                .map_err(|e| format!("Error dumping package list: {}", e))
            }));
    }
    Ok(())
}
