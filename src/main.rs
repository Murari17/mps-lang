#![deny(warnings)]

mod ast;
mod lexer;
mod parser;
mod codegen;
mod optimizer;
mod typechecker;

use lexer::Lexer;
use parser::Parser;
use codegen::Codegen;

use std::env;
use std::fs;
use std::io::{Read, Write, BufRead};
use std::path::{Path, PathBuf};
use std::process::Command;

const RUNTIME_H_CONTENT: &str = include_str!("runtime.h");

fn find_vcvars() -> Option<&'static str> {
    let vs_paths = [
        "C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\BuildTools\\VC\\Auxiliary\\Build\\vcvars64.bat",
        "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Auxiliary\\Build\\vcvars64.bat",
        "C:\\Program Files\\Microsoft Visual Studio\\2022\\Professional\\VC\\Auxiliary\\Build\\vcvars64.bat",
        "C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Auxiliary\\Build\\vcvars64.bat",
        "C:\\Program Files (x86)\\Microsoft Visual Studio\\2019\\BuildTools\\VC\\Auxiliary\\Build\\vcvars64.bat",
        "C:\\Program Files\\Microsoft Visual Studio\\2019\\Community\\VC\\Auxiliary\\Build\\vcvars64.bat",
        "C:\\Program Files\\Microsoft Visual Studio\\2019\\Professional\\VC\\Auxiliary\\Build\\vcvars64.bat",
        "C:\\Program Files\\Microsoft Visual Studio\\2019\\Enterprise\\VC\\Auxiliary\\Build\\vcvars64.bat",
    ];
    for path in &vs_paths {
        if Path::new(path).exists() {
            return Some(path);
        }
    }
    None
}

fn check_c_compiler() -> Option<&'static str> {
    if Command::new("gcc").arg("--version").output().is_ok() {
        Some("gcc")
    } else if Command::new("clang").arg("--version").output().is_ok() {
        Some("clang")
    } else if Command::new("cl").output().is_ok() {
        Some("cl")
    } else if find_vcvars().is_some() {
        Some("cl_vcvars")
    } else {
        None
    }
}

struct PythonPaths {
    include_dir: String,
    libs_dir: String,
    lib_name: String,
}

fn discover_python_paths() -> Option<PythonPaths> {
    let output = Command::new("python")
        .args([
            "-c",
            "import sys; import sysconfig; print(sys.executable + ';' + sysconfig.get_path('include') + ';' + str(sys.version_info.major) + ';' + str(sys.version_info.minor))"
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.trim().split(';').collect();
    if parts.len() < 4 {
        return None;
    }

    let exe_path = Path::new(parts[0]);
    let include_dir = parts[1].to_string();
    let major = parts[2];
    let minor = parts[3];

    let python_dir = exe_path.parent()?;
    
    #[cfg(target_os = "windows")]
    let libs_dir = python_dir.join("libs").to_string_lossy().into_owned();
    
    #[cfg(not(target_os = "windows"))]
    let libs_dir = python_dir.join("lib").to_string_lossy().into_owned();

    let lib_name = if cfg!(target_os = "windows") {
        format!("python{}{}", major, minor)
    } else {
        format!("python{}.{}", major, minor)
    };

    Some(PythonPaths {
        include_dir,
        libs_dir,
        lib_name,
    })
}

fn print_usage() {
    println!("Makes Python Slow (MPS) Compiler v0.1.0");
    println!("Usage:");
    println!("  mps [source_file.mps] [options]");
    println!("  mps [subcommand] [arguments]");
    println!();
    println!("Options:");
    println!("  --run                Compile and run the program immediately");
    println!("  --emit-c             Output transpiled C code (no compilation)");
    println!("  --emit-ast           Print parsed AST to stdout");
    println!("  --emit-so            Output a shared library (.dll or .so) instead of an executable");
    println!("  -o, --output <path>  Specify output binary path");
    println!("  -i, --repl           Start the interactive REPL shell");
    println!("  -h, --help           Show this help message");
    println!();
    println!("Subcommands:");
    println!("  lsp                  Start the stdio-based LSP server");
    println!("  format [files...]    Pretty-print and format the specified MPS files in place");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 {
        if args[1] == "lsp" {
            run_lsp();
            return;
        }
        if args[1] == "format" {
            run_formatter(&args[2..]);
            return;
        }
    }
    
    let mut source_path_str: Option<&String> = None;
    let mut interactive = false;
    let mut should_run = false;
    let mut emit_c = false;
    let mut emit_ast = false;
    let mut emit_so = false;
    let mut output_bin_path: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage();
                return;
            }
            "-i" | "--repl" => {
                interactive = true;
                i += 1;
            }
            "--run" => {
                should_run = true;
                i += 1;
            }
            "--emit-c" => {
                emit_c = true;
                i += 1;
            }
            "--emit-ast" => {
                emit_ast = true;
                i += 1;
            }
            "--emit-so" => {
                emit_so = true;
                i += 1;
            }
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_bin_path = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    eprintln!("Error: Missing path for --output option");
                    std::process::exit(1);
                }
            }
            arg => {
                if arg.starts_with('-') {
                    eprintln!("Error: Unknown option '{}'", arg);
                    print_usage();
                    std::process::exit(1);
                } else {
                    if source_path_str.is_some() {
                        eprintln!("Error: Multiple source files specified.");
                        print_usage();
                        std::process::exit(1);
                    }
                    source_path_str = Some(&args[i]);
                    i += 1;
                }
            }
        }
    }

    if source_path_str.is_none() {
        interactive = true;
    }

    if interactive {
        let mut initial_statements = Vec::new();
        let mut initial_source = String::new();

        if let Some(src_path_str) = source_path_str {
            let source_path = Path::new(src_path_str);
            if !source_path.exists() {
                eprintln!("Error: Source file '{}' does not exist.", src_path_str);
                std::process::exit(1);
            }
            let source_code = match fs::read_to_string(source_path) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("Error: Failed to read source file '{}': {}", src_path_str, e);
                    std::process::exit(1);
                }
            };
            let mut lexer = Lexer::new(&source_code);
            let tokens = match lexer.tokenize_all() {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };
            let mut parser = Parser::new(tokens);
            let program = match parser.parse_program() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };
            let mut typechecker = typechecker::TypeChecker::new(source_code.clone(), src_path_str.clone());
            if let Err(e) = typechecker.typecheck_program(&program) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
            initial_statements = program.statements;
            initial_source = source_code;
        }

        run_repl(initial_statements, initial_source);
        return;
    }

    // Non-interactive compilation flow
    let src_path_str = source_path_str.unwrap();
    let source_path = Path::new(src_path_str);
    if !source_path.exists() {
        eprintln!("Error: Source file '{}' does not exist.", src_path_str);
        std::process::exit(1);
    }

    // Determine output binary path if not specified
    let output_bin = output_bin_path.unwrap_or_else(|| {
        let mut p = source_path.to_path_buf();
        if emit_so {
            #[cfg(target_os = "windows")]
            p.set_extension("dll");
            #[cfg(not(target_os = "windows"))]
            p.set_extension("so");
        } else {
            #[cfg(target_os = "windows")]
            p.set_extension("exe");
            #[cfg(not(target_os = "windows"))]
            p.set_extension("");
        }
        p
    });

    // Check C compiler
    let c_compiler = check_c_compiler();

    // Read source code
    let source_code = match fs::read_to_string(source_path) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: Failed to read source file '{}': {}", src_path_str, e);
            std::process::exit(1);
        }
    };

    println!("[MPS] Lexing source...");
    let mut lexer = Lexer::new(&source_code);
    let tokens = match lexer.tokenize_all() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    println!("[MPS] Parsing AST...");
    let mut parser = Parser::new(tokens);
    let mut program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    // Resolve import / from-import statements
    println!("[MPS] Resolving imports...");
    let base_dir = source_path.parent().unwrap_or_else(|| Path::new("."));
    let mut already_imported: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Err(e) = resolve_imports(&mut program, base_dir, &mut already_imported) {
        eprintln!("{}", e);
        std::process::exit(1);
    }

    if emit_ast {
        println!("{:#?}", program);
        return;
    }

    println!("[MPS] Typechecking AST...");
    let mut typechecker = typechecker::TypeChecker::new(source_code.clone(), src_path_str.clone());
    if let Err(e) = typechecker.typecheck_program(&program) {
        eprintln!("{}", e);
        std::process::exit(1);
    }

    println!("[MPS] Optimizing AST...");
    program = optimizer::Optimizer::optimize(program);

    println!("[MPS] Transpiling to C11...");
    let mut codegen = Codegen::new();
    let c_code = match codegen.transpile_program(&program) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
    let has_py_imports = codegen.has_py_imports();

    // Determine directory where temporary files go (same directory as source)
    let parent_dir = source_path.parent().unwrap_or_else(|| Path::new("."));
    
    // Write runtime.h next to the output C file
    let runtime_h_path = parent_dir.join("runtime.h");
    if let Err(e) = fs::write(&runtime_h_path, RUNTIME_H_CONTENT) {
        eprintln!("Error: Failed to write runtime header: {}", e);
        std::process::exit(1);
    }

    let temp_c_path = source_path.with_extension("c");
    if let Err(e) = fs::write(&temp_c_path, &c_code) {
        eprintln!("Error: Failed to write transpiled C file: {}", e);
        std::process::exit(1);
    }

    // --emit-c: just output the C file and runtime.h, don't compile
    if emit_c {
        println!("[MPS] Transpilation Succeeded!");
        println!("[MPS] Generated C Code: {}", temp_c_path.display());
        println!("[MPS] Generated Runtime: {}", runtime_h_path.display());
        return;
    }

    if let Some(cc) = c_compiler {
        let compile_status = if cc == "cl" {
            println!("[MPS] Compiling C using MSVC (cl.exe) /O2...");
            let mut cmd = Command::new("cl");
            cmd.arg("/std:c11")
               .arg("/O2")
               .arg("/fp:fast")
               .arg(format!("/Fe:{}", output_bin.display()))
               .arg(&temp_c_path);
            
            if emit_so {
                cmd.arg("/LD").arg("/DMPS_EMIT_SO");
            }
            
            if has_py_imports {
                if let Some(paths) = discover_python_paths() {
                    cmd.arg(format!("/I{}", paths.include_dir));
                    cmd.arg("/link");
                    cmd.arg(format!("/LIBPATH:{}", paths.libs_dir));
                    cmd.arg(format!("{}.lib", paths.lib_name));
                } else {
                    eprintln!("[MPS] Warning: pyimport was used but CPython paths could not be discovered.");
                }
            }
            cmd.status()
        } else if cc == "cl_vcvars" {
            println!("[MPS] Compiling C using MSVC (via vcvars64.bat) /O2...");
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                let mut cmd = Command::new("cmd");
                let vcvars_path = find_vcvars().unwrap();
                let extra_flags = if emit_so { " /LD /DMPS_EMIT_SO" } else { "" };
                let mut cl_cmd = format!("call \"{}\" && cl /std:c11 /O2 /fp:fast{} /Fe:\"{}\" \"{}\"", vcvars_path, extra_flags, output_bin.display(), temp_c_path.display());
                if has_py_imports {
                    if let Some(paths) = discover_python_paths() {
                        cl_cmd.push_str(&format!(" /I\"{}\" /link /LIBPATH:\"{}\" \"{}.lib\"", paths.include_dir, paths.libs_dir, paths.lib_name));
                    } else {
                        eprintln!("[MPS] Warning: pyimport was used but CPython paths could not be discovered.");
                    }
                }
                cmd.raw_arg(format!("/c {}", cl_cmd));
                cmd.status()
            }
            #[cfg(not(windows))]
            {
                Err(std::io::ErrorKind::Unsupported.into())
            }
        } else {
            println!("[MPS] Compiling C using {} -O3...", cc);
            let mut cmd = Command::new(cc);
            cmd.arg("-O3")
               .arg("-ffast-math")
               .arg("-march=native")
               .arg(&temp_c_path)
               .arg("-o")
               .arg(&output_bin);

            if emit_so {
                cmd.arg("-shared").arg("-fPIC").arg("-DMPS_EMIT_SO");
            }

            if has_py_imports {
                if let Some(paths) = discover_python_paths() {
                    cmd.arg(format!("-I{}", paths.include_dir));
                    cmd.arg(format!("-L{}", paths.libs_dir));
                    cmd.arg(format!("-l{}", paths.lib_name));
                } else {
                    eprintln!("[MPS] Warning: pyimport was used but CPython paths could not be discovered.");
                }
            }
            cmd.status()
        };

        // Clean up temporary C file and header when compiled successfully
        let _ = fs::remove_file(&temp_c_path);
        let _ = fs::remove_file(&runtime_h_path);

        if cc == "cl" {
            // Clean up visual studio .obj object file
            let obj_path = temp_c_path.with_extension("obj");
            let _ = fs::remove_file(&obj_path);
        }

        match compile_status {
            Ok(status) if status.success() => {
                println!("[MPS] Build Succeeded: {}", output_bin.display());
                if should_run {
                    println!("[MPS] Executing binary...\n");
                    let run_bin = if output_bin.is_absolute() {
                        output_bin.clone()
                    } else if let Ok(abs) = std::fs::canonicalize(&output_bin) {
                        abs
                    } else {
                        Path::new(".").join(&output_bin)
                    };
                    let run_status = Command::new(&run_bin).status();
                    match run_status {
                        Ok(s) if s.success() => {}
                        Ok(s) => {
                            eprintln!("\n[MPS] Execution finished with exit code: {:?}", s.code());
                        }
                        Err(e) => {
                            eprintln!("Error: Failed to run binary: {}", e);
                        }
                    }
                }
            }
            _ => {
                eprintln!("Error: C compilation failed.");
                std::process::exit(1);
            }
        }
    } else {
        // No C compiler. We leave the C file and runtime.h so the user can compile it
        println!("[MPS] Transpilation Succeeded!");
        println!("[MPS] Generated C Code: {}", temp_c_path.display());
        println!("[MPS] Generated Runtime: {}", runtime_h_path.display());
        println!();
        println!("[MPS] Warning: No host C compiler (gcc, clang, or cl) was found in your PATH.");
        println!("      To compile to a native binary, please install GCC, Clang, or Visual Studio Build Tools.");
        println!("      Example (Windows Winget): winget install LLVM.LLVM");
        println!("      Example (Chocolatey):     choco install mingw");
    }
}

fn run_repl(initial_statements: Vec<ast::Stmt>, initial_source: String) {
    println!("Makes Python Slow (MPS) Interactive REPL v0.1.0");
    println!("Type \"exit\" or \"quit\" to exit.");
    println!();

    let mut session_statements = initial_statements;
    let mut session_source_code = initial_source;

    loop {
        let first_line = match read_line("mps>>> ") {
            Some(line) => line,
            None => break, // EOF
        };
        let mut input_str = first_line.trim_end().to_string();
        if input_str == "exit" || input_str == "quit" || input_str == "exit()" || input_str == "quit()" {
            break;
        }
        if input_str.is_empty() {
            continue;
        }

        // If it ends with ':', read more lines until an empty line is entered
        if input_str.ends_with(':') {
            loop {
                let next_line = match read_line("... ") {
                    Some(line) => line,
                    None => break,
                };
                let trimmed = next_line.trim_end();
                if trimmed.is_empty() {
                    break;
                }
                input_str.push('\n');
                input_str.push_str(trimmed);
            }
        }

        let mut lexer = Lexer::new(&input_str);
        let tokens = match lexer.tokenize_all() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
        };

        let mut parser = Parser::new(tokens);
        let new_program = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
        };

        if new_program.statements.is_empty() {
            continue;
        }

        let test_source = if session_source_code.is_empty() {
            input_str.clone()
        } else {
            format!("{}\n{}", session_source_code, input_str)
        };

        let combined_stmts = [session_statements.clone(), new_program.statements.clone()].concat();
        let test_program = ast::Program { statements: combined_stmts.clone() };

        let mut typechecker = typechecker::TypeChecker::new(test_source.clone(), "repl".to_string());
        if let Err(_) = typechecker.typecheck_program(&test_program) {
            // Error was already printed by typechecker's report_error
            continue;
        }

        let mut compiled_program = test_program;
        let last_idx = compiled_program.statements.len() - 1;
        if let ast::Stmt::ExprStmt(ref expr) = compiled_program.statements[last_idx] {
            if let Ok(expr_type) = typechecker.infer_expr_type(expr) {
                if expr_type != ast::Type::Void {
                    let is_print = match expr {
                        ast::Expr::Call { name, .. } => name == "print" || name == "mps_print" || name == "mps_println",
                        _ => false,
                    };
                    if !is_print {
                        let wrapped = ast::Expr::Call {
                            name: "print".to_string(),
                            args: vec![expr.clone()],
                        };
                        compiled_program.statements[last_idx] = ast::Stmt::ExprStmt(wrapped);
                    }
                }
            }
        }

        let mut codegen = Codegen::new();
        let c_code = match codegen.transpile_program(&compiled_program) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
        };
        let has_py_imports = codegen.has_py_imports();

        let temp_c_path = Path::new("_repl_temp.c");
        let temp_bin_path = if cfg!(target_os = "windows") {
            Path::new("_repl_temp.exe")
        } else {
            Path::new("_repl_temp")
        };
        let temp_header_path = Path::new("runtime.h");

        if let Err(e) = fs::write(temp_c_path, &c_code) {
            eprintln!("Error: Failed to write temp C file: {}", e);
            continue;
        }
        if let Err(e) = fs::write(temp_header_path, RUNTIME_H_CONTENT) {
            eprintln!("Error: Failed to write runtime header: {}", e);
            let _ = fs::remove_file(temp_c_path);
            continue;
        }

        let c_compiler = check_c_compiler();
        if c_compiler.is_none() {
            eprintln!("Error: No host C compiler (gcc, clang, or cl) was found in your PATH.");
            let _ = fs::remove_file(temp_c_path);
            let _ = fs::remove_file(temp_header_path);
            continue;
        }
        let cc = c_compiler.unwrap();

        let compile_output;
        if cc == "cl" {
            let mut compile_cmd = Command::new("cl");
            compile_cmd.arg("/std:c11")
                       .arg(format!("/Fe:{}", temp_bin_path.display()))
                       .arg(temp_c_path);
            if has_py_imports {
                if let Some(paths) = discover_python_paths() {
                    compile_cmd.arg(format!("/I{}", paths.include_dir));
                    compile_cmd.arg("/link");
                    compile_cmd.arg(format!("/LIBPATH:{}", paths.libs_dir));
                    compile_cmd.arg(format!("{}.lib", paths.lib_name));
                }
            }
            compile_output = compile_cmd.output();
        } else if cc == "cl_vcvars" {
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                let mut compile_cmd = Command::new("cmd");
                let vcvars_path = find_vcvars().unwrap();
                let mut cl_cmd = format!("call \"{}\" >nul && cl /std:c11 /Fe:\"{}\" \"{}\"", vcvars_path, temp_bin_path.display(), temp_c_path.display());
                if has_py_imports {
                    if let Some(paths) = discover_python_paths() {
                        cl_cmd.push_str(&format!(" /I\"{}\" /link /LIBPATH:\"{}\" \"{}.lib\"", paths.include_dir, paths.libs_dir, paths.lib_name));
                    }
                }
                compile_cmd.raw_arg(format!("/c {}", cl_cmd));
                compile_output = compile_cmd.output();
            }
            #[cfg(not(windows))]
            {
                compile_output = Err(std::io::ErrorKind::Unsupported.into());
            }
        } else {
            let mut compile_cmd = Command::new(cc);
            compile_cmd.arg(temp_c_path)
                       .arg("-o")
                       .arg(temp_bin_path);
            if has_py_imports {
                if let Some(paths) = discover_python_paths() {
                    compile_cmd.arg(format!("-I{}", paths.include_dir));
                    compile_cmd.arg(format!("-L{}", paths.libs_dir));
                    compile_cmd.arg(format!("-l{}", paths.lib_name));
                }
            }
            compile_output = compile_cmd.output();
        }

        let _ = fs::remove_file(temp_c_path);
        let _ = fs::remove_file(temp_header_path);
        if cc == "cl" || cc == "cl_vcvars" {
            let _ = fs::remove_file("_repl_temp.obj");
        }

        match compile_output {
            Ok(output) if output.status.success() => {
                let run_bin = if temp_bin_path.is_absolute() {
                    temp_bin_path.to_path_buf()
                } else if let Ok(abs) = std::fs::canonicalize(temp_bin_path) {
                    abs
                } else {
                    Path::new(".").join(temp_bin_path)
                };

                let run_status = Command::new(&run_bin).status();
                let _ = fs::remove_file(&run_bin);

                match run_status {
                    Ok(s) if s.success() => {
                        // Success! Update session statements and source
                        for stmt in new_program.statements {
                            let is_pure_print = match &stmt {
                                ast::Stmt::ExprStmt(ast::Expr::Call { name, .. }) => name == "print" || name == "mps_print" || name == "mps_println",
                                _ => false,
                            };
                            if !is_pure_print {
                                session_statements.push(stmt);
                            }
                        }
                        if session_source_code.is_empty() {
                            session_source_code = input_str;
                        } else {
                            session_source_code.push_str("\n");
                            session_source_code.push_str(&input_str);
                        }
                    }
                    Ok(s) => {
                        eprintln!("Execution finished with exit code: {:?}", s.code());
                    }
                    Err(e) => {
                        eprintln!("Error: Failed to run binary: {}", e);
                    }
                }
            }
            Ok(output) => {
                eprintln!("Error: C compilation failed.");
                use std::io::Write;
                let _ = std::io::stderr().write_all(&output.stdout);
                let _ = std::io::stderr().write_all(&output.stderr);
            }
            Err(e) => {
                eprintln!("Error: Failed to run C compiler: {}", e);
            }
        }
    }
}

fn read_line(prompt: &str) -> Option<String> {
    use std::io::{self, Write};
    print!("{}", prompt);
    let _ = io::stdout().flush();
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(0) => None,
        Ok(_) => Some(input),
        Err(_) => None,
    }
}

/// Resolve `import` and `from ... import` statements.
/// For each import, locate the .mps module, lex/parse it, recursively
/// resolve its own imports, then inline the imported definitions into
/// `program.statements`.
fn resolve_imports(
    program: &mut ast::Program,
    base_dir: &Path,
    already_imported: &mut std::collections::HashSet<String>,
) -> Result<(), String> {
    let mut injected: Vec<ast::Stmt> = Vec::new();

    for stmt in &program.statements {
        match stmt {
            ast::Stmt::Import { path, alias: _ } | ast::Stmt::FromImport { path, symbols: _ } => {
                // Build the module file path from the dotted path segments
                let relative = path.join(std::path::MAIN_SEPARATOR_STR);
                let module_file = format!("{}.mps", relative);

                // Search locations:
                //  1. Relative to source file
                //  2. stdlib/ directory next to the MPS compiler executable
                //  3. ~/.mps/packages/
                let candidate_relative = base_dir.join(&module_file);
                let candidate_stdlib = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join("stdlib").join(&module_file)));
                // Also try stdlib/ in the project root (for development)
                let candidate_stdlib_dev = base_dir.join("stdlib").join(&module_file);
                let candidate_global = dirs_candidate(&module_file);

                let resolved = if candidate_relative.exists() {
                    candidate_relative
                } else if candidate_stdlib_dev.exists() {
                    candidate_stdlib_dev
                } else if let Some(ref s) = candidate_stdlib {
                    if s.exists() { s.clone() } else if let Some(ref g) = candidate_global {
                        if g.exists() {
                            g.clone()
                        } else {
                            return Err(format!(
                                "MPS Error: Cannot find module '{}'. Searched:\n  - {}\n  - {}\n  - {}",
                                path.join("."),
                                candidate_relative.display(),
                                s.display(),
                                g.display()
                            ));
                        }
                    } else {
                        return Err(format!(
                            "MPS Error: Cannot find module '{}'. Searched:\n  - {}\n  - {}",
                            path.join("."),
                            candidate_relative.display(),
                            s.display(),
                        ));
                    }
                } else if let Some(ref g) = candidate_global {
                    if g.exists() {
                        g.clone()
                    } else {
                        return Err(format!(
                            "MPS Error: Cannot find module '{}'. Searched:\n  - {}\n  - {}",
                            path.join("."),
                            candidate_relative.display(),
                            g.display()
                        ));
                    }
                } else {
                    return Err(format!(
                        "MPS Error: Cannot find module '{}'. Searched:\n  - {}",
                        path.join("."),
                        candidate_relative.display(),
                    ));
                };

                let canonical = resolved.to_string_lossy().to_string();
                if already_imported.contains(&canonical) {
                    continue; // already loaded – skip to avoid cycles
                }
                already_imported.insert(canonical.clone());

                // Lex & parse the module
                let mod_source = fs::read_to_string(&resolved).map_err(|e| {
                    format!("MPS Error: Failed to read module '{}': {}", resolved.display(), e)
                })?;
                let mut mod_lexer = lexer::Lexer::new(&mod_source);
                let mod_tokens = mod_lexer.tokenize_all().map_err(|e| {
                    format!("MPS Error in module '{}': {}", path.join("."), e)
                })?;
                let mut mod_parser = parser::Parser::new(mod_tokens);
                let mut mod_program = mod_parser.parse_program().map_err(|e| {
                    format!("MPS Error in module '{}': {}", path.join("."), e)
                })?;

                // Recursively resolve the module's own imports
                let mod_dir = resolved.parent().unwrap_or(base_dir);
                resolve_imports(&mut mod_program, mod_dir, already_imported)?;

                // Determine which definitions to inject
                match stmt {
                    ast::Stmt::FromImport { symbols, .. } => {
                        // Only import the requested symbols
                        for mod_stmt in &mod_program.statements {
                            let name = stmt_name(mod_stmt);
                            if let Some(n) = name {
                                if symbols.contains(&n) {
                                    injected.push(mod_stmt.clone());
                                }
                            }
                        }
                    }
                    ast::Stmt::Import { .. } => {
                        // Import everything (functions, classes, traits, variables)
                        for mod_stmt in &mod_program.statements {
                            match mod_stmt {
                                ast::Stmt::FunctionDecl { .. }
                                | ast::Stmt::ClassDecl { .. }
                                | ast::Stmt::TraitDecl { .. }
                                | ast::Stmt::VariableDecl { .. } => {
                                    injected.push(mod_stmt.clone());
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // Prepend injected statements before original program body
    if !injected.is_empty() {
        injected.append(&mut program.statements);
        program.statements = injected;
    }

    Ok(())
}

/// Get the declared name of a statement (for selective from-import).
fn stmt_name(stmt: &ast::Stmt) -> Option<String> {
    match stmt {
        ast::Stmt::FunctionDecl { name, .. } => Some(name.clone()),
        ast::Stmt::ClassDecl { name, .. } => Some(name.clone()),
        ast::Stmt::TraitDecl { name, .. } => Some(name.clone()),
        ast::Stmt::VariableDecl { name, .. } => Some(name.clone()),
        _ => None,
    }
}

/// Build the global packages candidate path: ~/.mps/packages/<module_file>
fn dirs_candidate(module_file: &str) -> Option<std::path::PathBuf> {
    let home = if cfg!(target_os = "windows") {
        std::env::var("USERPROFILE").ok()
    } else {
        std::env::var("HOME").ok()
    };
    home.map(|h| Path::new(&h).join(".mps").join("packages").join(module_file))
}

fn run_lsp() {
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = std::io::stdout();

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            break;
        }
        if line.is_empty() {
            break;
        }
        if line.starts_with("Content-Length:") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 2 {
                continue;
            }
            let len: usize = parts[1].trim().parse().unwrap_or(0);
            
            let mut empty = String::new();
            let _ = reader.read_line(&mut empty);
            
            let mut body = vec![0; len];
            if reader.read_exact(&mut body).is_ok() {
                let body_str = String::from_utf8_lossy(&body);
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    let method = json["method"].as_str().unwrap_or("");
                    let id = &json["id"];
                    
                    match method {
                        "initialize" => {
                            let response = serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "capabilities": {
                                        "textDocumentSync": 1,
                                        "hoverProvider": true,
                                        "definitionProvider": true
                                    }
                                }
                            });
                            send_lsp_response(&mut stdout, response);
                        }
                        "textDocument/hover" => {
                            let params = &json["params"];
                            let doc_uri = params["textDocument"]["uri"].as_str().unwrap_or("");
                            let line = params["position"]["line"].as_u64().unwrap_or(0) as usize;
                            let character = params["position"]["character"].as_u64().unwrap_or(0) as usize;
                            
                            let hover_content = get_lsp_hover(doc_uri, line, character);
                            let response = serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": hover_content
                            });
                            send_lsp_response(&mut stdout, response);
                        }
                        "textDocument/definition" => {
                            let params = &json["params"];
                            let doc_uri = params["textDocument"]["uri"].as_str().unwrap_or("");
                            let line = params["position"]["line"].as_u64().unwrap_or(0) as usize;
                            let character = params["position"]["character"].as_u64().unwrap_or(0) as usize;
                            
                            let def_loc = get_lsp_definition(doc_uri, line, character);
                            let response = serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": def_loc
                            });
                            send_lsp_response(&mut stdout, response);
                        }
                        _ => {
                            if !id.is_null() {
                                let response = serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": serde_json::Value::Null
                                });
                                send_lsp_response(&mut stdout, response);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn send_lsp_response(stdout: &mut std::io::Stdout, response: serde_json::Value) {
    if let Ok(resp_str) = serde_json::to_string(&response) {
        let mut stdout_lock = stdout.lock();
        let _ = write!(stdout_lock, "Content-Length: {}\r\n\r\n{}", resp_str.len(), resp_str);
        let _ = stdout_lock.flush();
    }
}

fn token_len(st: &lexer::SpannedToken) -> usize {
    match &st.token {
        lexer::Token::Identifier(s) => s.len(),
        lexer::Token::IntLiteral(n) => n.to_string().len(),
        lexer::Token::FloatLiteral(f) => f.to_string().len(),
        lexer::Token::StringLiteral(s) => s.len() + 2,
        lexer::Token::FStringText(s) => s.len(),
        _ => 2,
    }
}

fn get_lsp_definition(uri: &str, line: usize, character: usize) -> serde_json::Value {
    let file_path = if uri.starts_with("file:///") {
        PathBuf::from(uri.trim_start_matches("file:///"))
    } else if uri.starts_with("file://") {
        PathBuf::from(uri.trim_start_matches("file://"))
    } else {
        PathBuf::from(uri)
    };

    let source = match fs::read_to_string(&file_path) {
        Ok(s) => s,
        Err(_) => return serde_json::Value::Null,
    };

    let mut lex = lexer::Lexer::new(&source);
    let tokens = match lex.tokenize_all() {
        Ok(t) => t,
        Err(_) => return serde_json::Value::Null,
    };

    let target_token = tokens.iter().find(|t| {
        t.line == line + 1 &&
        character + 1 >= t.col &&
        character + 1 < t.col + token_len(t)
    });

    let target_name = match target_token {
        Some(lexer::SpannedToken { token: lexer::Token::Identifier(name), .. }) => name,
        _ => return serde_json::Value::Null,
    };

    for i in 0..tokens.len() {
        let tok = &tokens[i];
        if let lexer::Token::Identifier(ref name) = tok.token {
            if name == target_name {
                let mut is_decl = false;
                if i > 0 {
                    match &tokens[i - 1].token {
                        lexer::Token::Fn | lexer::Token::Class | lexer::Token::Trait | lexer::Token::Let | lexer::Token::Const => {
                            is_decl = true;
                        }
                        _ => {}
                    }
                }
                if i > 1 && !is_decl {
                    if let (lexer::Token::Fn, lexer::Token::Async) = (&tokens[i - 1].token, &tokens[i - 2].token) {
                        is_decl = true;
                    }
                }
                
                if is_decl {
                    return serde_json::json!({
                        "uri": uri,
                        "range": {
                            "start": { "line": tok.line - 1, "character": tok.col - 1 },
                            "end": { "line": tok.line - 1, "character": tok.col - 1 + target_name.len() }
                        }
                    });
                }
            }
        }
    }

    serde_json::Value::Null
}

fn get_lsp_hover(uri: &str, line: usize, character: usize) -> serde_json::Value {
    let file_path = if uri.starts_with("file:///") {
        PathBuf::from(uri.trim_start_matches("file:///"))
    } else if uri.starts_with("file://") {
        PathBuf::from(uri.trim_start_matches("file://"))
    } else {
        PathBuf::from(uri)
    };

    let source = match fs::read_to_string(&file_path) {
        Ok(s) => s,
        Err(_) => return serde_json::Value::Null,
    };

    let mut lex = lexer::Lexer::new(&source);
    let tokens = match lex.tokenize_all() {
        Ok(t) => t,
        Err(_) => return serde_json::Value::Null,
    };

    let target_token = tokens.iter().find(|t| {
        t.line == line + 1 &&
        character + 1 >= t.col &&
        character + 1 < t.col + token_len(t)
    });

    let target_name = match target_token {
        Some(lexer::SpannedToken { token: lexer::Token::Identifier(name), .. }) => name,
        _ => return serde_json::Value::Null,
    };

    for i in 0..tokens.len() {
        let tok = &tokens[i];
        if let lexer::Token::Identifier(ref name) = tok.token {
            if name == target_name {
                let mut is_decl = false;
                if i > 0 {
                    match &tokens[i - 1].token {
                        lexer::Token::Fn | lexer::Token::Class | lexer::Token::Trait | lexer::Token::Let | lexer::Token::Const => {
                            is_decl = true;
                        }
                        _ => {}
                    }
                }
                if i > 1 && !is_decl {
                    if let (lexer::Token::Fn, lexer::Token::Async) = (&tokens[i - 1].token, &tokens[i - 2].token) {
                        is_decl = true;
                    }
                }

                if is_decl {
                    let lines: Vec<&str> = source.lines().collect();
                    if tok.line - 1 < lines.len() {
                        let raw_line = lines[tok.line - 1].trim();
                        let decl_str = if raw_line.ends_with(':') {
                            &raw_line[..raw_line.len() - 1]
                        } else {
                            raw_line
                        };
                        return serde_json::json!({
                            "contents": {
                                "kind": "markdown",
                                "value": format!("```mps\n{}\n```", decl_str)
                            }
                        });
                    }
                }
            }
        }
    }

    let builtins = [
        ("print", "fn print(x: any) -> void"),
        ("mps_print", "fn mps_print(x: any) -> void"),
        ("mps_println", "fn mps_println(x: any) -> void"),
        ("mps_input", "fn mps_input(prompt: string) -> string"),
        ("mps_to_int", "fn mps_to_int(x: any) -> int"),
        ("mps_to_float", "fn mps_to_float(x: any) -> float"),
        ("mps_to_string", "fn mps_to_string(x: any) -> string"),
        ("mps_to_bool", "fn mps_to_bool(x: any) -> bool"),
        ("len", "fn len(collection: PyObject) -> int"),
        ("range", "fn range(start: int, stop: int) -> PyObject"),
        ("matrix_add", "fn matrix_add(a: Matrix, b: Matrix) -> Matrix"),
        ("matrix_relu", "fn matrix_relu(a: Matrix) -> Matrix"),
        ("matrix32_add", "fn matrix32_add(a: Matrix32, b: Matrix32) -> Matrix32"),
        ("matrix32_relu", "fn matrix32_relu(a: Matrix32) -> Matrix32"),
        ("mps_random", "fn mps_random() -> float"),
        ("mps_randint", "fn mps_randint(min: int, max: int) -> int"),
        ("mps_random_seed", "fn mps_random_seed(seed: int) -> void"),
    ];

    for (b_name, b_sig) in &builtins {
        if *b_name == target_name {
            return serde_json::json!({
                "contents": {
                    "kind": "markdown",
                    "value": format!("```mps\n{}\n```", b_sig)
                }
            });
        }
    }

    serde_json::Value::Null
}

fn run_formatter(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: No files specified to format.");
        std::process::exit(1);
    }
    
    for filename in args {
        let path = Path::new(filename);
        if !path.exists() {
            eprintln!("Error: File '{}' does not exist.", filename);
            continue;
        }
        let source_code = match fs::read_to_string(path) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Error: Failed to read file '{}': {}", filename, e);
                continue;
            }
        };
        let mut lex = lexer::Lexer::new(&source_code);
        let tokens = match lex.tokenize_all() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Error in '{}': {}", filename, e);
                continue;
            }
        };
        let mut parser = parser::Parser::new(tokens);
        let program = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error in '{}': {}", filename, e);
                continue;
            }
        };
        let formatted = format_program(&program);
        if let Err(e) = fs::write(path, formatted) {
            eprintln!("Error: Failed to write to file '{}': {}", filename, e);
        } else {
            println!("Formatted '{}'", filename);
        }
    }
}

fn format_program(program: &ast::Program) -> String {
    let mut out = String::new();
    for stmt in &program.statements {
        out.push_str(&format_statement(stmt, 0));
    }
    out
}

fn format_statement(stmt: &ast::Stmt, indent: usize) -> String {
    let ind = "    ".repeat(indent);
    match stmt {
        ast::Stmt::FunctionDecl { name, params, return_type, body, is_async, decorators } => {
            let mut out = String::new();
            for dec in decorators {
                out.push_str(&format!("{}@{}\n", ind, dec));
            }
            let async_prefix = if *is_async { "async " } else { "" };
            let param_strs: Vec<String> = params.iter().map(|p| format!("{}: {}", p.name, p.param_type)).collect();
            out.push_str(&format!("{}fn {}{}({}) -> {}:\n", ind, async_prefix, name, param_strs.join(", "), return_type));
            for s in body {
                out.push_str(&format_statement(s, indent + 1));
            }
            out
        }
        ast::Stmt::ClassDecl { name, base_class, members } => {
            let mut out = String::new();
            let parent_suffix = if let Some(parent) = base_class { format!("({})", parent) } else { "".to_string() };
            out.push_str(&format!("{}class {}{}:\n", ind, name, parent_suffix));
            if members.is_empty() {
                out.push_str(&format!("{}    pass\n", ind));
            } else {
                for m in members {
                    out.push_str(&format_statement(m, indent + 1));
                }
            }
            out
        }
        ast::Stmt::TraitDecl { name, methods } => {
            let mut out = String::new();
            out.push_str(&format!("{}trait {}:\n", ind, name));
            for m in methods {
                let param_strs: Vec<String> = m.params.iter().map(|p| format!("{}: {}", p.name, p.param_type)).collect();
                out.push_str(&format!("{}    fn {}({}) -> {}\n", ind, m.name, param_strs.join(", "), m.return_type));
            }
            out
        }
        ast::Stmt::PyImport { library, alias } => {
            let alias_suffix = if let Some(a) = alias { format!(" as {}", a) } else { "".to_string() };
            format!("{}pyimport {}{}\n", ind, library, alias_suffix)
        }
        ast::Stmt::Import { path, alias } => {
            let alias_suffix = if let Some(a) = alias { format!(" as {}", a) } else { "".to_string() };
            format!("{}import {}{}\n", ind, path.join("."), alias_suffix)
        }
        ast::Stmt::FromImport { path, symbols } => {
            format!("{}from {} import {}\n", ind, path.join("."), symbols.join(", "))
        }
        ast::Stmt::VariableDecl { name, is_const, var_type, init } => {
            let keyword = if *is_const { "const" } else { "let" };
            let type_str = if let Some(t) = var_type { format!(": {}", t) } else { "".to_string() };
            let init_str = if let Some(expr) = init { format!(" = {}", format_expression(expr)) } else { "".to_string() };
            format!("{}{} {}{}{}\n", ind, keyword, name, type_str, init_str)
        }
        ast::Stmt::AssignStmt { lhs, value } => {
            format!("{}{} = {}\n", ind, format_expression(lhs), format_expression(value))
        }
        ast::Stmt::IfStmt { condition, then_branch, else_branch } => {
            let mut out = format!("{}if {}:\n", ind, format_expression(condition));
            for s in then_branch {
                out.push_str(&format_statement(s, indent + 1));
            }
            if let Some(eb) = else_branch {
                out.push_str(&format!("{}else:\n", ind));
                for s in eb {
                    out.push_str(&format_statement(s, indent + 1));
                }
            }
            out
        }
        ast::Stmt::WhileStmt { condition, body } => {
            let mut out = format!("{}while {}:\n", ind, format_expression(condition));
            for s in body {
                out.push_str(&format_statement(s, indent + 1));
            }
            out
        }
        ast::Stmt::ForStmt { var_name, iterable, body } => {
            let mut out = format!("{}for {} in {}:\n", ind, var_name, format_expression(iterable));
            for s in body {
                out.push_str(&format_statement(s, indent + 1));
            }
            out
        }
        ast::Stmt::TryCatchStmt { try_branch, catch_var, catch_branch, finally_branch } => {
            let mut out = format!("{}try:\n", ind);
            for s in try_branch {
                out.push_str(&format_statement(s, indent + 1));
            }
            out.push_str(&format!("{}catch {}:\n", ind, catch_var));
            for s in catch_branch {
                out.push_str(&format_statement(s, indent + 1));
            }
            if let Some(fb) = finally_branch {
                out.push_str(&format!("{}finally:\n", ind));
                for s in fb {
                    out.push_str(&format_statement(s, indent + 1));
                }
            }
            out
        }
        ast::Stmt::RaiseStmt(expr) => {
            format!("{}raise {}\n", ind, format_expression(expr))
        }
        ast::Stmt::MatchStmt { value, cases } => {
            let mut out = format!("{}match {}:\n", ind, format_expression(value));
            for case in cases {
                let pat_str = match &case.pattern {
                    ast::MatchPattern::Literal(lit) => format_literal(lit),
                    ast::MatchPattern::Wildcard => "_".to_string(),
                };
                out.push_str(&format!("{}    case {}:\n", ind, pat_str));
                for s in &case.body {
                    out.push_str(&format_statement(s, indent + 2));
                }
            }
            out
        }
        ast::Stmt::TupleUnpack { vars, init } => {
            format!("{}let {} = {}\n", ind, vars.join(", "), format_expression(init))
        }
        ast::Stmt::ExprStmt(expr) => {
            format!("{}{}\n", ind, format_expression(expr))
        }
        ast::Stmt::ReturnStmt(opt_expr) => {
            if let Some(expr) = opt_expr {
                format!("{}return {}\n", ind, format_expression(expr))
            } else {
                format!("{}return\n", ind)
            }
        }
        ast::Stmt::BreakStmt => format!("{}break\n", ind),
        ast::Stmt::ContinueStmt => format!("{}continue\n", ind),
    }
}

fn format_expression(expr: &ast::Expr) -> String {
    match expr {
        ast::Expr::Literal(lit) => format_literal(lit),
        ast::Expr::Identifier(name) => name.clone(),
        ast::Expr::Unary { op, operand } => format!("{}{}", op, format_expression(operand)),
        ast::Expr::Binary { op, left, right } => format!("({} {} {})", format_expression(left), op, format_expression(right)),
        ast::Expr::Call { name, args } => {
            let arg_strs: Vec<String> = args.iter().map(format_expression).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        ast::Expr::MemberAccess { object, member } => format!("{}.{}", format_expression(object), member),
        ast::Expr::MemberCall { object, method, args } => {
            let arg_strs: Vec<String> = args.iter().map(format_expression).collect();
            format!("{}.{}({})", format_expression(object), method, arg_strs.join(", "))
        }
        ast::Expr::OptionalMemberAccess { object, member } => format!("{}.?{}", format_expression(object), member),
        ast::Expr::OptionalMemberCall { object, method, args } => {
            let arg_strs: Vec<String> = args.iter().map(format_expression).collect();
            format!("{}.?{}({})", format_expression(object), method, arg_strs.join(", "))
        }
        ast::Expr::Subscript { object, index } => format!("{}[{}]", format_expression(object), format_expression(index)),
        ast::Expr::ListLiteral(exprs) => {
            let item_strs: Vec<String> = exprs.iter().map(format_expression).collect();
            format!("[{}]", item_strs.join(", "))
        }
        ast::Expr::DictLiteral(pairs) => {
            let pair_strs: Vec<String> = pairs.iter().map(|(k, v)| format!("{}: {}", format_expression(k), format_expression(v))).collect();
            format!("{{{}}}", pair_strs.join(", "))
        }
        ast::Expr::TupleLiteral(exprs) => {
            let item_strs: Vec<String> = exprs.iter().map(format_expression).collect();
            format!("({})", item_strs.join(", "))
        }
        ast::Expr::SuperCall { method, args } => {
            let arg_strs: Vec<String> = args.iter().map(format_expression).collect();
            format!("super.{}({})", method, arg_strs.join(", "))
        }
        ast::Expr::Lambda { params, return_type, body } => {
            let param_strs: Vec<String> = params.iter().map(|p| format!("{}: {}", p.name, p.param_type)).collect();
            format!("lambda {}: {} -> {}", param_strs.join(", "), return_type, format_expression(body))
        }
        ast::Expr::AwaitExpr(expr) => format!("await {}", format_expression(expr)),
        ast::Expr::FString { parts } => {
            let mut out = "f\"".to_string();
            for part in parts {
                match part {
                    ast::FStringPart::Text(t) => out.push_str(t),
                    ast::FStringPart::Expr(e) => out.push_str(&format!("{{{}}}", format_expression(e))),
                }
            }
            out.push('"');
            out
        }
        ast::Expr::Super => "super".to_string(),
        ast::Expr::Slice { object, start, end } => {
            let start_str = start.as_ref().map(|s| format_expression(s)).unwrap_or_default();
            let end_str = end.as_ref().map(|e| format_expression(e)).unwrap_or_default();
            format!("{}[{}:{}]", format_expression(object), start_str, end_str)
        }
        ast::Expr::ListComprehension { element, var_name, iterable } => {
            format!("[{} for {} in {}]", format_expression(element), var_name, format_expression(iterable))
        }
    }
}

fn format_literal(lit: &ast::Literal) -> String {
    match lit {
        ast::Literal::Int(n) => n.to_string(),
        ast::Literal::Float(f) => f.to_string(),
        ast::Literal::String(s) => format!("\"{}\"", s),
        ast::Literal::Bool(b) => b.to_string(),
        ast::Literal::Null => "null".to_string(),
    }
}

