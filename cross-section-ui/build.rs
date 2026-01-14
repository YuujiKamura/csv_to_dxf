fn main() {
    // sections.jsonが変更されたら再コンパイル
    println!("cargo:rerun-if-changed=../data/sections.json");
}
