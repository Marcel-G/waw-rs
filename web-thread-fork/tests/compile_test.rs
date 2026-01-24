#![cfg(not(target_family = "wasm"))]

use std::env;
use std::ffi::OsString;

use ui_test::custom_flags::rustfix::RustfixMode;
use ui_test::dependencies::DependencyBuilder;
use ui_test::status_emitter::Text;
use ui_test::{Args, Config, Format, OutputConflictHandling};

#[test]
fn test() {
	let mut config = Config {
		output_conflict_handling: OutputConflictHandling::Ignore,
		target: env::var_os("UI_TEST_TARGET").map(|target| target.into_string().unwrap()),
		..Config::rustc("tests/compile-fail")
	};
	let revisioned = config.comment_defaults.base();
	revisioned.set_custom("rustfix", RustfixMode::Disabled);

	let mut dependency_builder = DependencyBuilder::default();

	if let Some(flags) = env::var_os("UI_TEST_RUSTFLAGS").filter(|flags| !flags.is_empty()) {
		add_flags(&mut dependency_builder.program.envs, flags.clone());
		add_flags(&mut config.program.envs, flags);
	}

	if let Some(args) = env::var_os("UI_TEST_ARGS").filter(|args| !args.is_empty()) {
		let args = args.into_string().unwrap();

		for arg in args.split_ascii_whitespace() {
			dependency_builder.program.args.push(arg.into());
		}
	}

	if let Some(value) = env::var_os("UI_TEST_BUILD_STD").filter(|value| !value.is_empty()) {
		if value.eq_ignore_ascii_case("true") || value == "1" {
			dependency_builder.build_std = Some(String::from("panic_abort,std"));
		}
	}

	revisioned.set_custom("dependencies", dependency_builder);

	let args = Args::test().unwrap();
	#[allow(clippy::print_stdout)]
	if let Format::Pretty = args.format {
		println!(
			"Compiler: {}",
			config.program.display().to_string().replace('\\', "/")
		);
	}

	let text = match args.format {
		Format::Terse => Text::quiet(),
		Format::Pretty => Text::verbose(),
	};
	config.with_args(&args);

	ui_test::run_tests_generic(
		vec![config],
		ui_test::default_file_filter,
		ui_test::default_per_file_config,
		text,
	)
	.unwrap();
}

fn add_flags(envs: &mut Vec<(OsString, Option<OsString>)>, flags: OsString) {
	if let Some((_, current)) = envs.iter_mut().find(|(key, _)| key == "RUSTFLAGS") {
		if let Some(current) = current {
			current.push(" ");
			current.push(flags);
		} else {
			*current = Some(flags);
		}
	} else {
		envs.push((OsString::from("RUSTFLAGS"), Some(flags)));
	}
}
