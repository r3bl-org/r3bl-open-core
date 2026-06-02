fn main() {
    let message = "Hello, world!";
    println!("{}", message);
}

struct Person {
    name: String,
    age: u32,
}

impl Person {
    fn new(name: String, age: u32) -> Self {
        Person { name, age }
    }

    fn greet(&self) {
        println!("Hello, my name is {}", self.name);
    }
}

fn create_person() -> Person {
    Person::new("Alice".to_string(), 30)
}
