use sensitive_fmt::SensitiveDebug;

trait HasAssoc {
    type Assoc;
}

struct NotDebug;

struct Demo;

impl HasAssoc for Demo {
    type Assoc = NotDebug;
}

#[derive(SensitiveDebug)]
struct Wrapper<T>
where
    T: HasAssoc,
{
    value: <T as HasAssoc>::Assoc,
}

fn main() {
    let value = Wrapper::<Demo> { value: NotDebug };
    let _ = format!("{:?}", value);
}
