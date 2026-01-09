fn main() {
    glib_build_tools::compile_resources(
        &["data"],                          // <- slice, not single &str
        "data/resources.gresource.xml",     // path to your .gresource.xml
        "resources.gresource",             // output name inside OUT_DIR
    );
}
