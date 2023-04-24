fn main() {
    let ws: Vec<&str> = vec!["quick", "brown", "fox"];
    let xs: Vec<&&str> = ws.iter().collect::<Vec<_>>();
    let ys: Vec<(&str,)> = ws.iter().copied().map(|x| (x,)).collect::<Vec<_>>();
    let zs: Vec<String> = ws
        .iter()
        .copied()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    let wxs: Vec<(&str, &&str)> = ws
        .iter()
        .copied()
        .zip(xs.iter().copied())
        .collect::<Vec<_>>();
    let xys: Vec<(&&str, (&str,))> = xs
        .iter()
        .copied()
        .zip(ys.iter().copied())
        .collect::<Vec<_>>();
    let yzs: Vec<((&str,), String)> = ys
        .iter()
        .copied()
        .zip(zs.iter().cloned())
        .collect::<Vec<_>>();

    let _ = ws.iter().map(|w| *w == "").collect::<Vec<_>>();
    let _ = xs.iter().map(|x| **x == "").collect::<Vec<_>>();
    let _ = ys.iter().map(|y| y.0 == "").collect::<Vec<_>>();
    let _ = zs.iter().map(|z| z == "").collect::<Vec<_>>();

    let _ = wxs.iter().map(|wx| wx.0 == *wx.1).collect::<Vec<_>>();
    let _ = xys.iter().map(|xy| *xy.0 == xy.1 .0).collect::<Vec<_>>();
    let _ = yzs.iter().map(|yz| yz.0 .0 == yz.1).collect::<Vec<_>>();

    let _ = wxs.iter().map(|(w, x)| w == *x).collect::<Vec<_>>();
    let _ = xys.iter().map(|(x, y)| **x == y.0).collect::<Vec<_>>();
    let _ = yzs.iter().map(|(y, z)| y.0 == z).collect::<Vec<_>>();

    let _ = ws.clone().into_iter().map(|w| w == "").collect::<Vec<_>>();
    let _ = xs.clone().into_iter().map(|x| *x == "").collect::<Vec<_>>();
    let _ = ys
        .clone()
        .into_iter()
        .map(|y| y.0 == "")
        .collect::<Vec<_>>();
    let _ = zs.clone().into_iter().map(|z| z == "").collect::<Vec<_>>();

    let _ = wxs
        .clone()
        .into_iter()
        .map(|wx| wx.0 == *wx.1)
        .collect::<Vec<_>>();
    let _ = xys
        .clone()
        .into_iter()
        .map(|xy| *xy.0 == xy.1 .0)
        .collect::<Vec<_>>();
    let _ = yzs
        .clone()
        .into_iter()
        .map(|yz| yz.0 .0 == yz.1)
        .collect::<Vec<_>>();

    let _ = wxs
        .clone()
        .into_iter()
        .map(|(w, x)| w == *x)
        .collect::<Vec<_>>();
    let _ = xys
        .clone()
        .into_iter()
        .map(|(x, y)| *x == y.0)
        .collect::<Vec<_>>();
    let _ = yzs
        .clone()
        .into_iter()
        .map(|(y, z)| y.0 == z)
        .collect::<Vec<_>>();

    // smoelius: Additional reference possible.
    let _ = xs.iter().map(|&x| *x == "").collect::<Vec<_>>();
}

mod ref_necessity {
    #[derive(Clone, Copy)]
    struct X;

    fn foo(x: &X) {}

    fn bar() {
        let _ = [X].iter().map(|x| foo(x)).collect::<Vec<_>>();
        let _ = [&X].iter().map(|x| foo(x)).collect::<Vec<_>>();
    }
}

mod deref_impl {
    use std::ops::Deref;

    #[derive(Clone, Copy)]
    struct X(Y);

    #[derive(Clone, Copy)]
    struct Y;

    impl Deref for X {
        type Target = Y;

        fn deref(&self) -> &Y {
            &self.0
        }
    }

    fn foo() {
        let _ = [X(Y)].iter().map(|x| *x).collect::<Vec<_>>();
        let _ = [X(Y)].iter().map(|x| **x).collect::<Vec<_>>();
        // smoelius: Does not compile:
        // let _ = [X(Y)].iter().map(|&&x| x).collect::<Vec<_>>();
    }
}

fn tuple_with_wildcard() {
    let ws: Vec<&str> = vec!["quick", "brown", "fox"];
    let _ = ws.split_last().map(|(w, _)| *w);
    // smoelius: Does not compile:
    // let _ = ws.split_last().map(|&(w, _)| w);
}
