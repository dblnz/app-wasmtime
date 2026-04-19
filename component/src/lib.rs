wit_bindgen::generate!({
    // the name of the world in the `*.wit-files` input file
    world: "test",
});

struct Component;

impl Guest for Component {
    fn convert_celsius_to_fahrenheit(x: f32) -> f32 {
        x * (5 as f32 / 9 as f32) + 32f32
    }
}

export!(Component);
