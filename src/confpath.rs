use std::iter::{Iterator, FromIterator};
use std::rc::{Rc, Weak};
use std::fmt::{self, Display};
use std::cell::RefCell;
use std::collections::{HashMap, hash_map::Entry};
use std::hash::{Hash, Hasher};
use std::ops::Add;

#[derive(Debug)]
struct ConfPathData {
	name: Option<String>,
	parent: Weak<ConfPathData>,
	children: RefCell<HashMap<String, Rc<ConfPathData>>>
}

/// An owned, immutable configuration path.
///
/// This type provides methods like push and pop. None of these methods visibly
/// modifies `self`. All return a new `ConfPath` structure. This structure is
/// only a reference counted reference to the data of the path node.
/// Therefore it can be cloned without much overhead.
///
/// An [`iter()`](ConfPath::iter) method is provided for easy enumeration of the config paths
/// components.
///
/// # Examples
///
/// A configuration path can be built from an array of string references.
/// ```
/// use justconfig::ConfPath;
///
/// let cp = ConfPath::from(&["a", "b"]);
/// ```
///
/// Modifying a configuration path always returns a new path.
///
/// ```
/// use justconfig::ConfPath;
///
/// let cp = ConfPath::default();
/// let cp_a = cp.push("a");
///
/// assert_eq!(cp, ConfPath::default());
/// assert_eq!(cp_a, ConfPath::from(&["a"]));
/// ```
///
/// # Details
///
/// The ConfPath structure internally creates a tree of all config nodes that
/// where ever requested below the same root node. If you call `push` the new
/// value will be stored within the parent node until the whole configuration
/// tree gets torn down.
///
/// If you only want a temporary value, create a new configuration tree by
/// using the [`from`](ConfPath::from) method.
///
/// The comparison method `eq` makes sure, that the same paths from different
/// configuration trees compare equal. It uses a shortcut if the compared values
/// originate from the same configuration tree.
///
///
#[derive(Debug, Clone)]
pub struct ConfPath {
	data: Rc<ConfPathData>,
	root: Rc<ConfPathData>
}

impl Default for ConfPath {
	fn default() -> Self {
		let root_node = Rc::new(ConfPathData {
			name: None,
			parent: Weak::new(),
			children: RefCell::new(HashMap::default())
		});

		// The root node holds two references to itself.
		Self {
			data: root_node.clone(),
			root: root_node
		}
	}
}

impl Hash for ConfPath {
	fn hash<H: Hasher>(&self, state: &mut H) {
		for component in self.clone() {
			component.data.name.hash(state);
		}
	}
}

impl PartialEq for ConfPath {
	fn eq(&self, other: &Self) -> bool {
		// If the two elements point to the same data
		// they share the same config root and will definitely be equal.
		// This is ensured by the way `push` is implemented.
		if Rc::ptr_eq(&self.data, &other.data) {
			true
		} else {
			// If the root of the two ConfPath instances is the same
			// and the data pointers differ they are different. We do not
			// need to do more comparison.
			if Rc::ptr_eq(&self.root, &other.root) {
				false
			} else {
				// If the ConfPath instances do not share the same data and root
				// we fall back to comparing the path components.
				let mut s = self.clone();
				let mut o = other.clone();

				loop {
					match (s.pop(), o.pop()) {
						(Some((s_c_name, s_cp)), Some((o_c_name, o_cp))) if s_c_name == o_c_name => { s = s_cp; o = o_cp; } // Continue with the next part of the path
						(None, None) => break true,
						_ => break false
					}
				}
			}
		}
	}
}

impl Eq for ConfPath {
}

impl Add<&str> for ConfPath {
	type Output = Self;

	fn add(self, other: &str) -> Self {
		self.push(other)
	}
}

impl IntoIterator for ConfPath {
	type Item = ConfPath;
	type IntoIter = std::iter::Rev<std::vec::IntoIter<Self>>;

	fn into_iter(self) -> Self::IntoIter {
		let mut path = Vec::with_capacity(5);
		let mut pos = self;

		while !pos.is_root() {
			path.push(pos.clone());
			pos = pos.pop().unwrap().1;	// We already checked that this is not the root node. So unwrapping pop() is ok here.
		}

		path.into_iter().rev()
	}
}

impl Display for ConfPath {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let mut pos = self.clone();

		let mut add_delimiter = false;

		while !pos.is_root() {
			if add_delimiter {
				write!(f, ".")?;
			}

			// We already checked that this is not the root node. So unwrapping pop() and tail_component_name() is ok here.
			write!(f, "{}", pos.tail_component_name().unwrap())?;
			pos = pos.pop().unwrap().1;

			add_delimiter = true;
		}

		Ok(())
	}
}

impl <'a, T: AsRef<[&'a str]>> From<T> for ConfPath {
	fn from(components: T) -> Self {
		Self::default().push_all(components.as_ref())
	}
}

impl ConfPath {
	fn new(root: &Rc<ConfPathData>, data: Rc<ConfPathData>) -> Self {
		Self {
			data,
			root: root.clone()	// Increment the reference count on the root node
		}
	}

	/// Append a path component to this config path and return the new path.
	/// This path will not be modified.
	///
	/// # Example
	///
	/// ```
	/// use justconfig::ConfPath;
	///
	/// let cp_a = ConfPath::default().push("a");
	/// let cp_ab = cp_a.push("b");
	///
	/// assert_eq!(cp_ab, ConfPath::from(&["a", "b"]));
	/// ```
	pub fn push(&self, component: &str) -> Self {
		match self.data.children.borrow_mut().entry(component.to_owned()) {
			Entry::Occupied(child) => Self::new(&self.root, child.get().clone()),
			Entry::Vacant(child) => Self::new(&self.root, child.insert(Rc::new(ConfPathData {
				name: Some(component.to_owned()),
				parent: Rc::downgrade(&self.data),
				children: RefCell::new(HashMap::default())
			})).clone())
		}
	}

	/// Append multiple path components to this config path and return the new path.
	/// This path will not be modified.
	///
	/// # Example
	///
	/// ```
	/// use justconfig::ConfPath;
	///
	/// let cp_a = ConfPath::default().push("a");
	/// let cp_abc = cp_a.push_all(["b", "c"]);
	///
	/// assert_eq!(cp_abc, ConfPath::from(&["a", "b", "c"]));
	/// ```
	pub fn push_all<S: AsRef<str>, T: IntoIterator<Item = S>>(&self, iter: T) -> Self {
		iter.into_iter().fold(self.clone(), |prev, c| prev.push(c.as_ref()))
	}

	/// Remove the last component from this config path and return a new config path and the removed component.
	///
	/// The method returns a tuple containing an `Option` that stores the removed path component and a new config path containing the remaining path.
	/// If the config path is empty, the first element of the tuple is `None`.
	///
	/// # Example
	///
	/// ```
	/// use justconfig::ConfPath;
	///
	/// let cp = ConfPath::default().push_all(["a", "b"]);
	///
	/// let (component, cp) = cp.pop().unwrap();
	/// assert_eq!(component, "b");
	///
	/// let (component, cp) = cp.pop().unwrap();
	/// assert_eq!(component, "a");
	///
	/// assert!(cp.pop().is_none());
	/// ```
	pub fn pop(&self) -> Option<(&str, Self)> {
		if self.is_root() {
			None
		} else {
			let parent = self.data.parent.upgrade().unwrap();	// This is not the root node. So unwrap is ok here.

			Some((self.data.name.as_ref().unwrap(), Self::new(&self.root, parent)))	// Unwrap is ok here, because every node (execpt from the root node) must have a name.
		}
	}

	/// Checks if this ConfPath node is the root-node of the config path
	///
	/// This method returns true if this node is the root node of the config path.
	///
	/// # Example
	///
	/// ```
	/// use justconfig::ConfPath;
	///
	/// let cp = ConfPath::default();
	/// let cp_a = cp.push("a");
	///
	/// assert!(cp.is_root());
	/// assert!(!cp_a.is_root());
	/// ```
	pub fn is_root(&self) -> bool {
		// On the root node the data and the root pointer point to the same spot
		Rc::ptr_eq(&self.data, &self.root)
	}

	/// Returns the name of the last component of this config path.
	///
	/// If this method is called on the root of a ConfPath tree `None` is
	/// returned.
	///
	/// # Example
	///
	/// ```
	/// use justconfig::ConfPath;
	///
	/// let cp = ConfPath::default().push_all(["first", "second", "last"]);
	///
	/// assert_eq!(cp.tail_component_name().unwrap(), "last");
	/// ```
	pub fn tail_component_name(&self) -> Option<&str> {
		self.data.name.as_deref()
	}

	/// Returns an iterator that enumerates the components of the path.
	///
	/// The iterator returns the components first to last.
	/// Starting with the component directly below the root of the tree.
	pub fn iter(&self) -> impl Iterator<Item=Self> {
		self.clone().into_iter()
	}

	/// Returns an iterator that returns the children of the path element.
	///
	/// The children are not returned in any particular order.
	/// The iterator takes a snapshot of the current tree node. Therefore it's ok
	/// to update the config path while this iterator is used.
	///
	/// # Example
	///
	/// ```
	/// use justconfig::ConfPath;
	///
	/// let cp = ConfPath::default().push_all(["first", "second", "last"]);
	///
	/// for child in cp.children() {
	///   println!("{}", child.tail_component_name().unwrap());
	/// }
	/// ```
	pub fn children(&self) -> impl Iterator<Item=ConfPath> {
		Vec::from_iter(self.data.children.borrow().values().map(|v| ConfPath::new(&self.root, v.clone()))).into_iter()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::hash_set::HashSet;
	use std::collections::hash_map::DefaultHasher;

	fn check_path(cp: &ConfPath, components: &[&str]) {
		assert_eq!(cp.iter().zip(components.iter()).filter(|(l, &r)| l.tail_component_name().unwrap() == r).count(), components.len());
	}

	fn hash_pair(cp1: ConfPath, cp2: ConfPath) -> (u64, u64) {
		let mut hasher1 = DefaultHasher::new();
		let mut hasher2 = DefaultHasher::new();
		cp1.hash(&mut hasher1);
		cp2.hash(&mut hasher2);

		(hasher1.finish(), hasher2.finish())
	}

	#[test]
	fn creation() {
		let cp = ConfPath::default().push_all(["a", "b", "c"]);

		check_path(&cp, &["a", "b", "c"]);
	}

	#[test]
	fn pop() {
		let cp = ConfPath::default().push_all(["a", "b"]);

		// We do this manually to check that pop works correctly.
		let (part, cp) = cp.pop().unwrap();
		assert_eq!(part, "b");
		let (part, cp) = cp.pop().unwrap();
		assert_eq!(part, "a");

		assert!(cp.pop().is_none());
	}

	#[test]
	fn push() {
		let cp = ConfPath::default().push_all(["a", "b"]);

		let cp = cp.push("c");
		check_path(&cp, &["a", "b", "c"]);

		let cp = cp.push_all(["d", "e"]);
		check_path(&cp, &["a", "b", "c", "d", "e"]);
	}

	#[test]
	fn iterator() {
		let cp = ConfPath::default().push_all(["a", "b"]);

		let mut cp_iter = cp.into_iter();

		assert_eq!(cp_iter.next().unwrap().tail_component_name().unwrap(), "a");
		assert_eq!(cp_iter.next().unwrap().tail_component_name().unwrap(), "b");
		assert!(cp_iter.next().is_none());
		assert!(cp_iter.next().is_none());
	}
	
	#[test]
	fn is_root() {
		let cp_root = ConfPath::default();
		let cp_node = cp_root.push("a");

		assert!(cp_root.is_root());
		assert!(!cp_node.is_root());
	}

	#[test]
	fn add() {
		let cp = ConfPath::default();

		check_path(&(cp + "a"), &["a"]);
	}

	#[test]
	fn comparison() {
		let root1 = ConfPath::default();
		let root2 = ConfPath::default();

		// Make sure root nodes always compare equal
		assert_eq!(root1, root1);
		assert_eq!(root2, root2);
		assert_eq!(root1, root2);

		// Make sure the same strings compare equal
		assert_eq!(root1.push("a"), root1.push("a"));
		assert_eq!(root1.push_all(["a", "b"]), root1.clone() + "a" + "b");

		// Make sure different paths do not compare equal
		assert_ne!(root1.push_all(["a", "b"]), root1.push("a"));
		assert_ne!(root1.push_all(["a", "b"]), root1.push("b"));

		// Make sure the same path in different roots compares equal
		assert_eq!(root1.push("a"), root2.push("a"));
		assert_eq!(root1.push_all(["a", "b"]), root2.push_all(["a", "b"]));

		// Make sure that different paths in different roots do not compare equal
		assert_ne!(root1.push("a"), root2.push("b"));
		assert_ne!(root1.push_all(["a", "b"]), root2.push_all(["a", "b", "c"]));
	}

	#[test]
	fn hash() {
		let cp = ConfPath::default();

		// Check that the same path creates the same hash
		let (h1, h2) = hash_pair(cp.push_all(["a", "b"]), cp.push_all(["a", "b"]));
		assert_eq!(h1, h2);

		let (h1, h2) = hash_pair(cp.push_all(["a", "b", "c"]), cp.push_all(["a", "b", "c"]));
		assert_eq!(h1, h2);

		// Check that all values are used for a hash
		let (h1, h2) = hash_pair(cp.push_all(["a", "b"]), cp.push_all(["a"]));
		assert_ne!(h1, h2);

		let (h1, h2) = hash_pair(cp.push_all(["a", "b"]), cp.push_all(["b"]));
		assert_ne!(h1, h2);

		// Check that there is no length extension problem
		let (h1, h2) = hash_pair(cp.push_all(["a", "b", "c"]), cp.push_all(["a", "bc"]));
		assert_ne!(h1, h2);
	}

	#[test]
	fn free() {
		// This weak reference is used to test if the tree is freed correctly after
		// the last ConfPath was dropped.
		let wr_root;
		let wr_inode;

		{
			let lnode;

			{
				// Create the following tree:
				// root -> internal -> leaf
				// The ConfPaths referencing to `root` and `internal` will be
				// dropped after this inner scope.
				let root = ConfPath::default();
				let inode = root.push("internal");
				lnode = inode.push("leaf");

				wr_root = Rc::downgrade(&root.data);
				wr_inode = Rc::downgrade(&inode.data);

				assert!(wr_root.upgrade().is_some());
				assert!(wr_inode.upgrade().is_some());
			}

			// Now `root` and `internal` are dropped. The reference to the
			// path component `leaf` must keep the whole tree and all of its
			// children alive.
			lnode.push("test");

			assert!(wr_root.upgrade().is_some());
			assert!(wr_inode.upgrade().is_some());
	}

		// Now even `leave` was dropped. The root node and all it's children
		// must be gone now!
		assert!(wr_root.upgrade().is_none());
		assert!(wr_inode.upgrade().is_none());
	}

	#[test]
	fn enum_children() {
		let cp = ConfPath::default();
		cp.push("a");
		cp.push("b");
		cp.push_all(["a", "a1"]);

		// The order of the returned elements is not guaranteed. Therefore we've
		// to remove the returned elements form the reference_set and later
		// check if the set is empty.
		#[allow(clippy::mutable_key_type)] // This is one of the false positives mentioned in the documentation.
		let mut reference_set: HashSet<ConfPath> = HashSet::from_iter([ConfPath::from(&["a"]), ConfPath::from(&["b"])].iter().cloned());
		cp.children().for_each(|c| assert!(reference_set.remove(&c), "Iterator returned too many elements."));
		assert_eq!(reference_set.len(), 0, "Iterator returned not enough elements.");

		// Verify again with an intermediate node
		#[allow(clippy::mutable_key_type)] // This is one of the false positives mentioned in the documentation.
		let mut reference_set: HashSet<ConfPath> = HashSet::from_iter([ConfPath::from(&["a", "a1"])].iter().cloned());
		cp.push("a").children().for_each(|c| assert!(reference_set.remove(&c), "Iterator returned too many elements."));
		assert_eq!(reference_set.len(), 0, "Iterator returned not enough elements.");
	}

	#[test]
	fn enum_children_const() {
		let cp = ConfPath::default();
		cp.push("a");
		cp.push("b");

		let root_child_iter = cp.children();

		// Push another element and verify the iterator will not return it
		cp.push("d");

		// The order of the returned elements is not guaranteed. Therefore we've
		// to remove the returned elements form the reference_set and later
		// check if the set is empty.
		#[allow(clippy::mutable_key_type)] // This is one of the false positives mentioned in the documentation.
		let mut reference_set: HashSet<ConfPath> = HashSet::from_iter([ConfPath::from(&["a"]), ConfPath::from(&["b"])].iter().cloned());
		root_child_iter.for_each(|c| assert!(reference_set.remove(&c), "Iterator returned to many elements."));
		assert_eq!(reference_set.len(), 0, "Iterator returned not enough elements.");
	}
}
