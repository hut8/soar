extern crate ogn_parser;

fn main() {
    let result = ogn_parser::parse(
        r"PUR64020B>OGNPUR,qAS,PureTrk23:/142436h4546.60N/01146.10Eg166.56186289668/018/A=002753 !W64! id1E64020B +000fpm +0.0rot 0.0dB 0e +0.0kHz gps2x3",
    );

    println!("Data source: {:?}", result.as_ref().unwrap().data_source());
    println!("{result:#?}");
}
