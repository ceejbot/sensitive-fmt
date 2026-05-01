use sensitive_fmt::SensitiveDebug;

trait HasAssoc {
    type Assoc;
}

struct NotDisplay;

struct Demo;

impl HasAssoc for Demo {
    type Assoc = NotDisplay;
}

#[derive(SensitiveDebug)]
struct Wrapper<T>
where
    T: HasAssoc,
{
    #[sensitive(truncate = 4)]
    value: <T as HasAssoc>::Assoc,
}

fn main() {
    let value = Wrapper::<Demo> { value: NotDisplay };
    let _ = format!("{:?}", value);
}
