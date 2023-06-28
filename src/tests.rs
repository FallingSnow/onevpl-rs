fn examples() {
    let t = trycmd::TestCases::new();
    t.register_bins(trycmd::cargo::compile_examples([]).unwrap());
    t.case("examples/*.md");
}