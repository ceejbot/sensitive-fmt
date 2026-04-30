use sensitive_fmt::SensitiveDisplay;

trait HasAssoc {
    type Assoc;
}

struct NotDisplay;

struct Demo;

impl HasAssoc for Demo {
    type Assoc = NotDisplay;
}

#[derive(SensitiveDisplay)]
struct Wrapper<T>
where
    T: HasAssoc,
{
    value: <T as HasAssoc>::Assoc,
}

fn main() {
    let value = Wrapper::<Demo> { value: NotDisplay };
    let _ = format!("{}", value);
}
