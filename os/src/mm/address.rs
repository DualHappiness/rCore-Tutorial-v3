use crate::config::{PAGE_SIZE, PAGE_SIZE_BITS};
use core::{
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
};

use super::page_table::PageTableEntry;

#[derive(Clone, Copy, Ord, PartialEq, PartialOrd, Eq, Debug)]
pub struct Physical;
#[derive(Clone, Copy, Ord, PartialEq, PartialOrd, Eq, Debug)]
pub struct Virtual;
#[derive(Clone, Copy, Ord, PartialEq, PartialOrd, Eq, Debug)]
pub struct Address;
#[derive(Clone, Copy, Ord, PartialEq, PartialOrd, Eq, Debug)]
pub struct PageNumber;

#[derive(Clone, Copy, Ord, PartialEq, PartialOrd, Eq)]
pub struct Warpper<S, T>(pub usize, PhantomData<S>, PhantomData<T>);
impl<S, T> From<usize> for Warpper<S, T> {
    fn from(v: usize) -> Self {
        Self(v, PhantomData, PhantomData)
    }
}
impl<S, T> From<Warpper<S, T>> for usize {
    fn from(v: Warpper<S, T>) -> Self {
        v.0
    }
}

pub type Addr<T> = Warpper<Address, T>;
pub type PageNum<T> = Warpper<PageNumber, T>;
impl<T> Addr<T> {
    pub fn floor(&self) -> PageNum<T> {
        (self.0 / PAGE_SIZE).into()
    }
    pub fn ceil(&self) -> PageNum<T> {
        ((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE).into()
    }
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}
impl<T> From<Addr<T>> for PageNum<T> {
    fn from(v: Addr<T>) -> Self {
        assert_eq!(v.page_offset(), 0);
        v.floor()
    }
}
impl<T> From<PageNum<T>> for Addr<T> {
    fn from(v: PageNum<T>) -> Self {
        (v.0 << PAGE_SIZE_BITS).into()
    }
}

pub type PhysAddr = Addr<Physical>;
pub type PhysPageNum = PageNum<Physical>;
impl Debug for PhysAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("PA:{:#x}", self.0))
    }
}
impl Debug for PhysPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("PPN:{:#x}", self.0))
    }
}

impl PhysPageNum {
    pub fn get_pte_array(&self) -> &'static mut [PageTableEntry] {
        let pa: PhysAddr = self.clone().into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, 512) }
    }
    pub fn get_bytes_array(&self) -> &'static mut [u8] {
        let pa: PhysAddr = self.clone().into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut u8, PAGE_SIZE) }
    }
    pub fn get_mut<T>(&self) -> &'static mut T {
        let pa: PhysAddr = self.clone().into();
        unsafe { (pa.0 as *mut T).as_mut().unwrap() }
    }
    pub fn get_mut_offset<T>(&self, offset: usize) -> &'static mut T {
        let pa: PhysAddr = self.clone().into();
        let pa = pa.0 + offset;
        unsafe { (pa as *mut T).as_mut().unwrap() }
    }
}

pub type VirtAddr = Addr<Virtual>;
pub type VirtPageNum = PageNum<Virtual>;
impl Debug for VirtAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("VA:{:#x}", self.0))
    }
}
impl Debug for VirtPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("VPN:{:#x}", self.0))
    }
}
impl VirtPageNum {
    pub fn indexes(&self) -> [usize; 3] {
        let mut vpn = self.0;
        let mut idx = [0usize; 3];
        for i in (0..3).rev() {
            idx[i] = vpn & 0b111_111_111;

            vpn >>= 9;
        }
        idx
    }
}

pub trait StepByOne {
    fn step(&mut self);
}

impl StepByOne for VirtPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    l: T,
    r: T,
}

impl<T> SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(l: T, r: T) -> Self {
        assert!(l <= r, "start {:?} > end {:?}!", l, r);
        Self { l, r }
    }
    pub fn get_start(&self) -> T {
        self.l
    }
    pub fn get_end(&self) -> T {
        self.r
    }
}

pub struct SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    current: T,
    end: T,
}

impl<T> SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(l: T, r: T) -> Self {
        Self { current: l, end: r }
    }
}

impl<T> Iterator for SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let t: T = self.current;
            self.current.step();
            Some(t)
        }
    }
}

impl<T> IntoIterator for SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;

    type IntoIter = SimpleRangeIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter::new(self.l, self.r)
    }
}

pub type VPNRange = SimpleRange<VirtPageNum>;
