# vapi-rust

Let's try and connect to Varnish using rust.

# using it
Edit your Cargo.toml and add:

    [dependencies.vapi]
    path = "/PATH/TO/vapi-rust"

Hopefully, once it is usable, it'll go to crates.io

Then you can print the logs:

    let vd = VsmData::new(VsmType::Default)).unwrap();
    vd.log();

# Know issues
Naming is hard, and I suck at it, therefore, some names may not make sense, and camel case may or may not be used when it should or should not be used.

In the same spirit, the organization of file is a bit messy, that'll be cleaned later once the API is frozen.

Vsm.Data segfaults, I know, I haven't figured that one out yet, if you have some pointers, I'd be happy to learn!
