use std::{mem::size_of, collections::{HashMap, hash_map::Entry}};
use rand::Rng;
use tarpc::{
	context,
	serde::Serialize,
	serde::Deserialize
};
use log::{info, warn, debug};


type Digest = u64;
// number of bits
const NUM_BITS: usize = size_of::<Digest>() * 8;

// Data part of the node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
	pub id: Digest,
	pub addr: String
}

#[tarpc::service]
pub trait NodeService {
	async fn get_node_rpc() -> Node;
	async fn get_predecessor_rpc() -> Option<Node>;
	async fn get_successor_rpc() -> Option<Node>;

	async fn find_successor_rpc(id: Digest) -> Node;
	async fn find_predecessor_rpc(id: Digest) -> Node;
	async fn closest_preceding_finger_rpc(id: Digest) -> Node;
	async fn notify_rpc(node: Node);
	async fn stabilize_rpc();
}

#[derive(Clone)]
pub struct NodeServer {
	node: Node,
	successor: Option<Node>,
	predecessor: Option<Node>,
	finger_table: [Option<Node>; NUM_BITS as usize],
	// connection to remote nodes
	connection_map: HashMap<Digest, NodeServiceClient>
}

impl NodeServer {
	pub fn new(node: &Node) -> NodeServer {
		// init a ring with only one node
		// (see second part of n.join in Figure 6)
		const INIT_FINGER: Option<Node> = None;
		let mut finger_table = [INIT_FINGER; NUM_BITS];
		for i in 0..NUM_BITS {
			finger_table[i] = Some(node.clone());
		}

		NodeServer {
			node: node.clone(),
			successor: Some(node.clone()),
			predecessor: Some(node.clone()),
			finger_table: finger_table,
			connection_map: HashMap::new()
		}
	}

	// Calculate start field of finger table (see Table 1)
	// k in [0, m)
	pub fn finger_table_start(&self, k: usize) -> u64 {
		(self.node.id + (1 << k)) % (NUM_BITS as u64)
	}
	
	async fn get_connection(&mut self, node: &Node) -> &NodeServiceClient {
		if node.id == self.node.id {
			panic!("Node {} connecting to itself", node.id);
		}

		match self.connection_map.entry(node.id) {
			Entry::Occupied(c) => c.into_mut(),
			// connect to the node
			Entry::Vacant(m) => {
				info!("Connecting from node {} to node {}", self.node.id, node.id);
				let c = crate::client::setup_client(&node.addr).await;
				info!("Connected from node {} to node {}", self.node.id, node.id);
				m.insert(c)
			}
		}
	}

	// Figure 7: n.join
	pub async fn join(&mut self, node: &Node) {
		debug!("Node {}: joining node {}", self.node.id, node.id);
		self.predecessor = None;
		let n = self.get_connection(node).await;
		self.successor = Some(n.find_successor_rpc(context::current(), node.id).await.unwrap());
		debug!("Node {}: joined node {}", self.node.id, node.id);
	}

	// Figure 7: n.stabilize
	pub async fn stabilize(&mut self) {
		let ctx = context::current();
		let successor = match self.successor.as_ref() {
			Some(s) => {
				// Skip if the successor is self
				if s.id == self.node.id {
					return;
				}
				s.clone()
			},
			None => {
				warn!("Empty successor");
				return;
			}
		};

		let self_node = self.node.clone();
		let n= self.get_connection(&successor).await;
		let x= match n.get_predecessor_rpc(ctx).await.unwrap() {
			Some(v) => v,
			None => {
				warn!("Empty predecessor of successor node: {:?}", successor);
				return;
			}
		};
		n.notify_rpc(ctx, self_node).await.unwrap();

		if x.id > self.node.id && x.id < successor.id {
			self.successor = Some(x);
		}
	}

	// Figure 7: n.fix_fingers
	pub async fn fix_fingers(&mut self) {
		let mut rng = rand::thread_rng();
		let i = rng.gen_range(1..NUM_BITS);
		self.finger_table[i] = Some(self.find_successor(self.finger_table_start(i)).await);
	}

	// Figure 4: n.find_successor
	async fn find_successor(&mut self, id: Digest) -> Node {
		debug!("Node {}: finding predecessor of {}", self.node.id, id);
		let n = self.find_predecessor(id).await;
		if n.id == self.node.id {
			return self.successor.as_ref().unwrap().clone()
		}
		let node = self.get_connection(&n).await;
		node.get_successor_rpc(context::current()).await.unwrap().unwrap()
	}

	// Figure 4: n.find_predecessor
	async fn find_predecessor(&mut self, id: Digest) -> Node {
		let mut n = self.node.clone();
		let mut succ = self.successor.as_ref().expect("empty succussor").clone();

		while id > n.id && id < succ.id {
			let node = self.get_connection(&n).await;
			n = node.closest_preceding_finger_rpc(context::current(), id).await.unwrap();
			let new_node = self.get_connection(&n).await;
			succ = new_node.get_successor_rpc(context::current()).await.unwrap().unwrap_or_else(|| panic!("Empty succussor for node {:?}", new_node));
		}
		n
	}

	// Figure 4: n.closest_preceding_finger
	async fn closest_preceding_finger(&mut self, id: Digest) -> Node {
		for i in (0..NUM_BITS).rev() {
			match self.finger_table[i].as_ref() {
				Some(n) => if n.id > id && n.id < self.node.id {
					return n.clone();
				},
				None => ()
			};
		}
		self.node.clone()
	}

	// Figure 7: n.notify
	async fn notify(&mut self, node: Node) {
		let new_pred = match self.predecessor.as_ref() {
			Some(v) => if node.id > v.id && node.id < self.node.id {
				node
			} else {
				v.clone()
			},
			None => node
		};
		self.predecessor = Some(new_pred);
	}
}

#[tarpc::server]
impl NodeService for NodeServer {
	async fn get_node_rpc(self, _: context::Context) -> Node {
		debug!("Node {}: get_node_rpc called", self.node.id);
		let node = self.node.clone();
		debug!("Node {}: get_node_rpc called", self.node.id);
		node
	}

	async fn get_predecessor_rpc(self, _: context::Context) -> Option<Node> {
		debug!("Node {}: get_predecessor_rpc called", self.node.id);
		let pred = self.predecessor.clone();
		debug!("Node {}: get_predecessor_rpc finished", self.node.id);
		pred
	}

	async fn get_successor_rpc(self, _: context::Context) -> Option<Node> {
		debug!("Node {}: get_successor_rpc called", self.node.id);
		let succ = self.successor.clone();
		debug!("Node {}: get_successor_rpc finished", self.node.id);
		succ
	}

	async fn find_successor_rpc(mut self, _: context::Context, id: Digest) -> Node {
		debug!("Node {}: find_successor_rpc called", self.node.id);
		let succ = self.find_successor(id).await;
		debug!("Node {}: find_successor_rpc finished", self.node.id);
		succ
	}

	async fn find_predecessor_rpc(mut self, _: context::Context, id: Digest) -> Node {
		debug!("Node {}: find_predecessor_rpc called", self.node.id);
		let pred = self.find_predecessor(id).await;
		debug!("Node {}: find_predecessor_rpc finished", self.node.id);
		pred
	}

	async fn closest_preceding_finger_rpc(mut self, _: context::Context, id: Digest) -> Node {
		debug!("Node {}: closest_preceding_finger_rpc called", self.node.id);
		let node = self.closest_preceding_finger(id).await;
		debug!("Node {}: closest_preceding_finger_rpc finished", self.node.id);
		node
	}

	async fn notify_rpc(mut self, _: context::Context, node: Node) {
		debug!("Node {}: notify_rpc called", self.node.id);
		self.notify(node).await;
		debug!("Node {}: notify_rpc finished", self.node.id);
	}

	async fn stabilize_rpc(mut self, _: context::Context) {
		debug!("Node {}: stabilize_rpc called", self.node.id);
		self.stabilize().await;
		debug!("Node {}: stabilize_rpc finished", self.node.id);
	}
}
