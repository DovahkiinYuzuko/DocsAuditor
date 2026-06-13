fn add(a: i32, b: i32) -> i32 {
    let _ = subtract(a, b);
    a + b
}

fn subtract(a: i32, b: i32) -> i32 {
    a - b
}

fn dummy() {
    let _ = add(1, 2);
}
