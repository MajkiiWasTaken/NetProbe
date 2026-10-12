/************************************************
* File: output.rs
* Author: Michal Švrček
*
* Colored terminal output
*
* ver. 0.1.0
*************************************************/
use colored::Colorize;
pub fn banner() {
    println!(
        "\n{}",
        format!("◆ NetProbe v{}", env!("CARGO_PKG_VERSION"))
            .bright_blue()
            .bold()
    );
    println!("{}", "────────────────────────────────────────".dimmed());
}
pub fn heading(s: &str) {
    println!("\n{}", s.cyan().bold());
}
pub fn info(s: &str) {
    println!("{} {}", "[INFO]".blue().bold(), s);
}
pub fn ok(s: &str) {
    println!("{} {}", "[ OK ]".green().bold(), s);
}
pub fn warn(s: &str) {
    println!("{} {}", "[WARN]".yellow().bold(), s);
}
pub fn error(s: &str) {
    eprintln!("{} {}", "[ERROR]".red().bold(), s);
}
