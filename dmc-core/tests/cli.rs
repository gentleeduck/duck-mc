use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn init_writes_config() {
  let dir = tempfile::tempdir().unwrap();
  let cfg = dir.path().join("dmc.toml");
  Command::cargo_bin("dmc").unwrap().args(["init", "--path"]).arg(&cfg).assert().success();
  let body = std::fs::read_to_string(&cfg).unwrap();
  assert!(body.contains("[[collections]]"));
  assert!(body.contains("docs"));
}

#[test]
fn init_refuses_overwrite() {
  let dir = tempfile::tempdir().unwrap();
  let cfg = dir.path().join("dmc.toml");
  std::fs::write(&cfg, "x").unwrap();
  Command::cargo_bin("dmc").unwrap().args(["init", "--path"]).arg(&cfg).assert().failure();
}

#[test]
fn build_runs_engine() {
  let dir = tempfile::tempdir().unwrap();
  let docs = dir.path().join("docs");
  std::fs::create_dir_all(&docs).unwrap();
  std::fs::write(docs.join("a.mdx"), "---\ntitle: T\n---\n# A\nbody\n").unwrap();
  let out_dir = dir.path().join("out");
  let cfg = format!(
    "output_dir = {:?}\n\n[[collections]]\nname = \"docs\"\npattern = \"docs/**/*.mdx\"\nbase_dir = {:?}\n",
    out_dir,
    dir.path()
  );
  let cfg_path = dir.path().join("dmc.toml");
  std::fs::write(&cfg_path, cfg).unwrap();
  Command::cargo_bin("dmc")
    .unwrap()
    .args(["build", "--config"])
    .arg(&cfg_path)
    .assert()
    .success()
    .stdout(contains("docs — 1 records"));
  assert!(out_dir.join("docs.json").exists());
}

#[test]
fn compile_prints_json_for_a_file() {
  let dir = tempfile::tempdir().unwrap();
  let f = dir.path().join("a.mdx");
  std::fs::write(&f, "# Hi").unwrap();
  let assert = Command::cargo_bin("dmc").unwrap().args(["compile"]).arg(&f).assert().success();
  let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
  let _: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
}
