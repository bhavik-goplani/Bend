use clap::{Parser, ValueEnum};
use hvm_lang::{check_book, compile_book, load_file_to_book, run_book, RunInfo};
use hvmc::{
  ast::{show_book, show_net},
  run::Ptr,
};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
  #[arg(value_enum)]
  pub mode: Mode,

  #[arg(short, long)]
  pub verbose: bool,

  #[arg(long, help = "How much memory to allocate for the runtime", default_value = "1G", value_parser = mem_parser)]
  pub mem: usize,

  #[arg(help = "Path to the input file")]
  pub path: PathBuf,
}

#[derive(ValueEnum, Clone, Debug)]
enum Mode {
  /// Checks that the program is sintactically and semantically correct.
  Check,
  /// Compiles the program to hvmc and prints to stdout.
  Compile,
  /// Compiles the program and runs it with the hvm.
  Run,
}

fn mem_parser(arg: &str) -> Result<usize, String> {
  let (base, mult) = match arg.to_lowercase().chars().last() {
    None => return Err("Mem size argument is empty".to_string()),
    Some('k') => (&arg[0 .. arg.len() - 1], 1 << 10),
    Some('m') => (&arg[0 .. arg.len() - 1], 1 << 20),
    Some('g') => (&arg[0 .. arg.len() - 1], 1 << 30),
    Some(_) => (arg, 1),
  };
  let base = base.parse::<usize>().map_err(|e| e.to_string())?;
  Ok(base * mult)
}

fn main() -> Result<(), String> {
  #[cfg(not(feature = "cli"))]
  compile_error!("The 'cli' feature is needed for the hvm-lang cli");

  let args = Args::parse();

  let mut book = load_file_to_book(&args.path)?;
  if args.verbose {
    println!("{book:?}");
  }

  match args.mode {
    Mode::Check => {
      check_book(book)?;
    }
    Mode::Compile => {
      let (compiled, _) = compile_book(&mut book)?;
      println!("{}", show_book(&compiled));
    }
    Mode::Run => {
      let mem_size = args.mem / std::mem::size_of::<(Ptr, Ptr)>();
      let (res_term, def_names, info) = run_book(book, mem_size)?;
      let RunInfo { stats, valid_readback, net: lnet } = info;
      let rps = stats.rewrites.total_rewrites() as f64 / stats.run_time / 1_000_000.0;
      if args.verbose {
        println!("\n{}", show_net(&lnet));
      }

      if valid_readback {
        println!("{}\n", res_term.to_string(&def_names));
      } else {
        println!("Invalid readback from inet.");
        println!("Got:\n{}\n", res_term.to_string(&def_names));
      }
      // TODO: check for -s flag
      println!(
        "RWTS   : {}",
        stats.rewrites.anni
          + stats.rewrites.comm
          + stats.rewrites.eras
          + stats.rewrites.dref
          + stats.rewrites.oper
      );
      println!("- ANNI : {}", stats.rewrites.anni);
      println!("- COMM : {}", stats.rewrites.comm);
      println!("- ERAS : {}", stats.rewrites.eras);
      println!("- DREF : {}", stats.rewrites.dref);
      println!("- OPER : {}", stats.rewrites.oper);
      println!("TIME   : {:.3} s", stats.run_time);
      println!("RPS    : {:.3} m", rps);
    }
  }

  Ok(())
}
