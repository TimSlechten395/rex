use std::ops::Range;

mod context;
//
use rand::rngs::ThreadRng;
//
// a memory region is what is required to build a runtime type every runtime type is of the form
// MemoryRegion -> Type
struct MemoryRegion {
    // multiple non_connected regions: just needs to be a list of all memory addresses
    // Vec<Range<_>> is a performent way to store it
    region: Vec<Range<usize>>,
    // Memory is modelled as being fractional. You do not always have full ownership you sometimes
    // only have a part of it.
    // There are multiple ways of dividing memory logically.
    // This implementation only allows splitting memory exactly in half,
    // it stores (n1, n2) representing n1/(n2^2) of the memory.
    // To create a new type you need full access but partial access can be used to read. The
    // specific amount of access is stored only for the purpose of merging regions back together
    // TODO: ideally ownership should be only there at runtime in complex cases and sometimes only at
    // compiletime. Maybe some special way of generics needs to be used here like ~const (usize,
    // usize). Another way of splitting ownership is splitting of an infinitesimal part that cannot
    // be split further and then you keep track of all the pieces that are split of from the main
    // one. this is essentialy reference counting.
    ownership: (usize, usize),
}
//
// TypeId0 is a memory region
struct TypeId(usize);

// wait how would this work in our system. i32 is a (MemoryRegion with region.size() = 32 and ownership = (1, 1)) -> Self  and String is also a
// (MemoryRegion with region.size() = 64 (or usize) and ownership = (1, 1)) -> Self and there is a hidden which also needs a MemoryRegion
pub struct Person {
    pub age: i32,
    pub name: String,

    // wait for private fields the size should still be public (if this is a problem just box it)
    inner_id: i32,
}

trait PersonAbstract {
    // This is the main eliminator there might be others
    // type is linear so memoryRegion is returned as well it should have a wherebound which states
    // which memory Regions are there
    fn age_and_name(self, i32_destructor: impl Fn(i32)) -> (i32, String, MemoryRegion);
    // these methods could be defined but they would take a method fo destructing the other parts
    // these are just helper methods no intended to be overriden? maybe i want these methods to be
    // guarenteed zero cost field access? Maybe some kind of cost annotation should be added to
    // functions
    // fn age(StringDestructor: String -> MemoryRegion) -> (i32, MemoryRegion);
    // fn name(i32Destructor: i32 -> MemoryRegion) -> (String, MemoryRegion);
    //
    //
    // This is the interesting part you know an id went in and the age_and_name destructor shows
    // that it is still there but you cannot get it out. Adding a private field means adding
    // destructor as parameter which means it is a breaking change. To prevent this all possible
    // destructors should be passed in and be clonable?
    //
    fn new(age: i32, name: String, id: i32) -> Self;

    // other methods that take a full person
}

// // define a type by its constructors and eliminators only
// struct Type {
//     typeid: TypeId,
//     // these are the arguments to the constructor
//     cons: Vec<Vec<TypeId>>,
//     // These are the output of the
//     elims: Vec<Vec<TypeId>>,
// }
//
// fn my_u32() -> Type {
//     Type {
//         typeid: TypeId(1),
//         // this should somehow be for all usize x: x..(x + 32)
//         cons: vec![vec![MemoryRegion(0..32)]],
//         //wait do elims even make sense shouldnt the elims line up with construction more?
//         elims: vec![vec![MemoryRegion(0..32)]],
//     }
// }
//
// // fn pair() -> Type {
// //     Type {
// //         typeid: TypeId(3),
// //         cons: vec![vec![TypeId(1), TypeId(2)]],
// //         elims: vec![vec![4]],
// //     }
// // }
//
