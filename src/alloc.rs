
#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;


use libamber::sparsevec::SparseVec;

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();
    let mut xs: SparseVec<isize> = (0..10).collect::<Vec<_>>().into();
    xs.append(&xs.clone());
    xs.insert(1000000, 5);
    println!("{:?}", xs);
}
