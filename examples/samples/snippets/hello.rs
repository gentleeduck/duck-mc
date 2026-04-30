use std::io::Write;

fn main() {
  let name = "dmc";
  println!("hello, {name}!");
  let _ = std::io::stdout().flush();
  greet(name);
}

fn greet(target: &str) {
  println!("greetings, {target}");
}
