trait Doer {
    fn do_it(&self, it: String);
}

struct Thing<'a, T: Doer> {
    doer: &'a T,
}

impl<'a, T> Thing<'a, T>
where
    T: Doer,
{
    fn new(doer: &'a T) -> Self {
        Thing { doer }
    }

    fn do_it(&self, it: String) {
        self.doer.do_it(it)
    }
}

struct NormalDoer {}

impl Doer for NormalDoer {
    fn do_it(&self, it: String) {
        println!("{}", it)
    }
}

fn main() {
    let my_thing = Thing::new(&NormalDoer {});
    my_thing.do_it("hello".to_string())
}

#[cfg(test)]
mod test {
    use std::cell::RefCell;

    use super::*;

    struct MockDoer {
        do_reqs: RefCell<Vec<String>>,
    }

    impl Doer for MockDoer {
        fn do_it(&self, it: String) {
            self.do_reqs.borrow_mut().push(it)
        }
    }

    #[test]
    fn test_thing() {
        let mock_doer = MockDoer {
            do_reqs: RefCell::new(Vec::new()),
        };

        let test_thing = Thing::new(&mock_doer);
        test_thing.do_it("hello".to_string());

        assert_eq!(vec!["hello".to_string()], *mock_doer.do_reqs.borrow());
    }
}
