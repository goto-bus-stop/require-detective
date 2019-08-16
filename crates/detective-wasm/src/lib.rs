use wasm_bindgen::prelude::*;

fn convert_err(err: require_detective::Error) -> JsValue {
    JsValue::from_str(&format!("{}", err))
}

#[wasm_bindgen]
pub fn find(source: &str) -> Result<JsValue, JsValue> {
    require_detective::find(source).map(|found| JsValue::from_serde(&found).unwrap()).map_err(convert_err)
}

#[wasm_bindgen]
pub fn detective(source: &str) -> Result<JsValue, JsValue> {
    require_detective::detective(source)
        .map(|list| JsValue::from_serde(&list).unwrap())
        .map_err(convert_err)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
