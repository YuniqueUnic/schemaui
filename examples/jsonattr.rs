use schemars::JsonSchema;

#[derive(JsonSchema)]
#[schemars(extend("simple" = "string value",))]
struct Struct {
    #[schemars(extend("widget" ="color-picker"))]
    color: String,
}

fn main() {
    println!("Hello, world!");
    let x = schemars::schema_for!(Struct);
    println!("{}", serde_json::to_string_pretty(&x).unwrap());
}
