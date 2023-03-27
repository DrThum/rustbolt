use dbc_extractor::extract;

fn main() -> Result<(), std::io::Error> {
    extract("patch-frFR-2.MPQ", "DBFilesClient\\ChrRaces.dbc")
}
