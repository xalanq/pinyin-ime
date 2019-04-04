macro_rules! join {
    ( < $x:ident , ) => {
        $x
    };
    ( < $x:ident , $( $xs:ident , )+ ) => {
        ( $x , join!( < $( $xs , )+ ) )
    };

    ( > $x:expr , ) => {
        ($x)()
    };
    ( > $x:expr , $( $xs:expr , )+ ) => {
        ::rayon::join( $x , || join!( > $( $xs , )+ ) )
    };

    ( @ $( let $lhs:ident = $rhs:expr ; )+ ) => {
        {
            let join!( < $( $lhs , )+ ) = join!( > $( $rhs , )+ );
            ($( $lhs ),+) // flattened tuple
        }
    };
    ( @ $( let $lhs:ident = $rhs:expr ; )* $x:expr $( , $xs:expr )*) => {
        join! { @
            $( let $lhs = $rhs ; )*
            let lhs = $x;
            $($xs),*
        }
    };

    ( $x:expr $( , $xs:expr )* ) => {
        join! { @ $x $( , $xs )* }
    };
}
