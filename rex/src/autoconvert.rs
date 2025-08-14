// use std::{
//     any::{Any, TypeId},
//     collections::{HashMap, HashSet, VecDeque},
//     marker::PhantomData,
// };
//
// // TODO: unify the ConvertIdMap and the CallerMap
//
// use crate::context::Context;
//
// struct Convert<S, T> {
//     func: fn(S) -> T,
// }
//
// struct AutoConvert<S, T> {
//     _marker: PhantomData<(S, T)>,
// }
//
// pub struct ConversionGraph {
//     graph: HashMap<TypeId, Vec<TypeId>>,
// }
//
// // This is a lookup table for gertting the id of Convert<S, T> if you have the id for S and T
// pub struct ConvertIdMap {
//     map: HashMap<(TypeId, TypeId), TypeId>,
// }
//
// impl ConvertIdMap {
//     fn new() -> Self {
//         Self {
//             map: HashMap::new(),
//         }
//     }
// }
//
// impl ConversionGraph {
//     fn new() -> Self {
//         Self {
//             graph: HashMap::new(),
//         }
//     }
//
//     fn add_edge(&mut self, src: TypeId, dst: TypeId) {
//         self.graph.entry(src).or_default().push(dst);
//     }
//
//     fn bfs(&self, src: TypeId, dst: TypeId) -> Option<Vec<TypeId>> {
//         let mut queue = VecDeque::new();
//         let mut visited = HashSet::new();
//         let mut came_from = HashMap::new();
//
//         queue.push_back(src);
//         visited.insert(src);
//
//         while let Some(mut cur) = queue.pop_front() {
//             if cur == dst {
//                 let mut path = vec![cur];
//
//                 while let Some(&prev) = came_from.get(&cur) {
//                     cur = prev;
//                     path.push(cur);
//                 }
//                 path.reverse();
//                 return Some(path);
//             }
//
//             if let Some(neighbors) = self.graph.get(&cur) {
//                 for &neighbor in neighbors {
//                     if !visited.contains(&neighbor) {
//                         visited.insert(neighbor);
//                         came_from.insert(neighbor, cur);
//                         queue.push_back(neighbor);
//                     }
//                 }
//             }
//         }
//         None
//     }
// }
//
// fn autoconvert_to_i32() -> Convert<Box<dyn Any>, i32> {
//     Convert { func: |_blanket| 0 }
// }
//
// // This function includes autoconversion for now
// fn insert_conversion<S: 'static + Clone, T: 'static>(
//     context: &mut Context,
//     id_map: &mut ConvertIdMap,
//     graph: &mut ConversionGraph,
//     callers: &mut HashMap<TypeId, ConversionCaller>,
//     convert: Convert<S, T>,
// ) {
//     context.insert(convert);
//     let ids = (TypeId::of::<S>(), TypeId::of::<T>());
//     let conv_id = TypeId::of::<Convert<S, T>>();
//     graph.add_edge(ids.0, ids.1);
//     id_map.map.insert(ids, conv_id);
//
//     // I dont get this
//     callers.insert(conv_id, |boxed_convert, boxed_input| {
//         let convert = boxed_convert.downcast_ref::<Convert<S, T>>()?;
//         let input = boxed_input.downcast_ref::<S>()?;
//         let result = (convert.func)(input.clone());
//         Some(Box::new(result))
//     });
// }
//
// // This is a hack because rust reflection is limited;
// type ConversionCaller = fn(&dyn Any, &dyn Any) -> Option<Box<dyn Any>>;
//
// fn convert(
//     callers: &HashMap<TypeId, ConversionCaller>,
//     conv_id: TypeId,
//     context: &Context,
//     from: &dyn Any,
// ) -> Box<dyn Any> {
//     let boxed_convert = context.map.get(&conv_id).unwrap();
//     let caller = callers.get(&conv_id).unwrap();
//     caller(boxed_convert.as_ref(), from)
// }
//
// #[test]
// fn test_bfs_conversion() {
//     let mut graph = ConversionGraph::new();
//
//     let mut context = Context::new();
//     let mut id_map = ConvertIdMap::new();
//     let mut callers: HashMap<TypeId, ConversionCaller> = HashMap::new();
//
//     let u8_to_u16 = Convert {
//         func: |u8: u8| u16::from(u8),
//     };
//
//     let u16_to_u32 = Convert {
//         func: |u8: u16| u32::from(u8),
//     };
//
//     insert_conversion(
//         &mut context,
//         &mut id_map,
//         &mut graph,
//         &mut callers,
//         u8_to_u16,
//     );
//     insert_conversion(
//         &mut context,
//         &mut id_map,
//         &mut graph,
//         &mut callers,
//         u16_to_u32,
//     );
//
//     let path = graph
//         .bfs(TypeId::of::<u8>(), TypeId::of::<u32>())
//         .expect("Path not found");
//
//     let conversions = path.windows(2).map(|ids| {
//         id_map
//             .map
//             .get(&(*ids.first().unwrap(), *ids.get(1).unwrap()))
//     });
//
//     let mut next = Box::new(34) as Box<dyn Any>;
//     for conv in conversions {
//         next = convert(
//             &callers,
//             *conv.expect("Invalid Conversion function"),
//             &context,
//             &next,
//         )
//         .expect("");
//     }
//
//     assert_eq!(34, *next.downcast::<u32>().unwrap())
// }
