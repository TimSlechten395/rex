use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
};

use super::MemoryRegion;

///------------------------ Compiler as a library ---------------------------------------------------
// There is no real macro system if you want macros you can just use the compiler as a library

/// ------------------------ Context ---------------------------------------------------
/// The Context system is not a high priority and can be build inside the language itself with
/// monads
pub struct Context {
    context: HashMap<TypeId, Box<dyn Any>>,
}

// maybe traits should have a context as well maybe be generic over it?
// Without a context there might be no way to drop the residue.
// pub trait Convert<T> {
//     fn convert(self) -> T;
// }
//
// pub trait Coercion<T>: Convert {
//     fn coerce(self) -> T {
//         self.convert()
//     }

// controls priority lower means higher priority
// This system might be flawed see next trait for alternative
// const PRIORITY: u32 = 100;
// }

// This has the advantage that there are no arbitrary priority values but now this trait can only
// be implemented for your own types so coercion will only work in one way.
// the best way might be to have both the trait above and this one where this one specifies the
// order and if a coercion is not inside the list then it will have a priority of 0
// pub trait CoercionPriority {
// first vec is the order second is for if multiple coercions should have the same priority
// typeid is not good enough here some dependent type magic will be necesarry
// fn list() -> Vec<Vec<TypeId>>;
// }

// ------------------------ TRAITS ---------------------------------------------------
// interfaces are very similar like traits in rust apart from a couple differences.
// 1. Traits are implemented by which traits are currently in the context. This allows
//    overwriteing traits locally. specialization is also a given because of the coercion
//    mechanism
// 2. returning an opaque impl trait is returning a sigma type (T, MyTrait<T>)
// 3. There is no implicit Self Parameter. For rust like traits you should add at least one
//    generic type. Traits without any generics are used for the  module system.
// 4. impl blocks put things inside the context. 'pub impl' puts things inside the main function
//    and has an orphan rule (passing things to the main function at compile time is a common pattern).
//
//    Private 'impl' blocks (if implemented) override the context for pub functions
//    inside the module (This is like a module level macro).

// trait bounds could be just sugar for requiring vtable structs inside the context
// pub fn hello<T: Debug>(debug_object: T)
// this will become (normally object_debug_trait is inside the context):
// pub fn hello<T>(debug_ojbect: T, object_debug_trait<T> )

// vtable example
// For direct implementation it is pretty simple you just create a struct and put it in the context
// other implemtations are of the form T: ... -> PersonAbstract<T> This can work if we have
// coercions and orders for coercion. With coercion alone it already works by picking the specific
// type PersonAbstract<MyPerson> first and then the coercions and if we have order defined properly
// it also handles order between the coercions properly for example an implementations with a trait
// bound take priority over those who have no/fewer bounds. This is like specialization in rust but
// fully
// What about default implementations? we cannot just pass a copy for each type
//
// struct PersonAbstract<T> {
//     age_and_name: fn(T, impl Fn(i32)) -> (i32, String, MemoryRegion),
//     new: fn(i32, String, i32) -> T,
// }
//

// fn number_parser<'a>() -> impl Parser<'a, &'a str, BigUint, extra::Err<Rich<'a, char>>> {
//     text::digits(10)
//         .to_slice()
//         .map(|x| BigUint::from_str(x).unwrap())
//     // let digits = text::digits(10).to_slice();
//     //
//     // let frac = just('.').then(digits);
//     //
//     // let exp = just('e')
//     //     .or(just('E'))
//     //     .then(one_of("+-").or_not())
//     //     .then(digits);
//     //
//     // just('-')
//     //     .or_not()
//     //     .then(text::int(10))
//     //     .then(frac.or_not())
//     //     .then(exp.or_not())
//     //     .to_slice()
//     //     .map(|s: &str| s.parse().unwrap())
//     //     .boxed()
// }
