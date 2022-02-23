//! Integration tests for the `tree_memory_arena` module.
use std::{
  sync::Arc,
  thread::{self, JoinHandle},
};

/// Rust book: https://doc.rust-lang.org/book/ch11-03-test-organization.html#the-tests-directory
use r3bl_rs_utils::{
  tree_memory_arena::{Arena, MTArena, ResultUidList},
  utils::{style_primary, style_prompt},
};

#[test]
fn test_can_add_nodes_to_tree() {
  // Can create an arena.
  let mut arena = Arena::<usize>::new();
  let node_1_value = 42 as usize;
  let node_2_value = 100 as usize;

  // Can insert a node - node_1.
  {
    let node_1_id = arena.add_new_node(node_1_value, None);
    assert_eq!(node_1_id, 0);
  }

  // Can find node_1 by id.
  {
    let node_1_id = 0;
    assert!(arena.get_node_arc(node_1_id).is_some());

    let node_1_ref = dbg!(arena.get_node_arc(node_1_id).unwrap());
    let node_1_ref_weak = arena.get_node_arc_weak(node_1_id).unwrap();
    assert_eq!(node_1_ref.read().unwrap().payload, node_1_value);
    assert_eq!(
      node_1_ref_weak.upgrade().unwrap().read().unwrap().payload,
      42
    );
  }

  // Can't find node by id that doesn't exist.
  {
    let node_id_dne = 200 as usize;
    assert!(arena.get_node_arc(node_id_dne).is_none());
  }

  // Can add child to node_1.
  {
    let node_1_id = 0 as usize;
    let node_2_id = arena.add_new_node(node_2_value, Some(node_1_id));
    let node_2_ref = dbg!(arena.get_node_arc(node_2_id).unwrap());
    let node_2_ref_weak = arena.get_node_arc_weak(node_2_id).unwrap();
    assert_eq!(node_2_ref.read().unwrap().payload, node_2_value);
    assert_eq!(
      node_2_ref_weak.upgrade().unwrap().read().unwrap().payload,
      node_2_value
    );
  }

  // Can dfs tree walk.
  {
    let node_1_id = 0 as usize;
    let node_2_id = 1 as usize;

    let node_list = dbg!(arena.tree_walk_dfs(node_1_id).unwrap());

    assert_eq!(node_list.len(), 2);
    assert_eq!(node_list, vec![node_1_id, node_2_id]);
  }
}

#[test]
fn test_can_walk_tree_and_delete_nodes_from_tree() {
  let mut arena = Arena::<String>::new();

  // root
  //   +- child1
  //   |    +- gc1
  //   |    +- gc2
  //   +- child2

  let root = arena.add_new_node("root".to_string(), None);
  let child1 = arena.add_new_node("child1".to_string(), Some(root));
  let gc_1_id = arena.add_new_node("gc1".to_string(), Some(child1));
  let gc_2_id = arena.add_new_node("gc2".to_string(), Some(child1));
  let child_2_id = arena.add_new_node("child2".to_string(), Some(root));
  println!("{}, {:#?}", style_primary("arena"), arena);

  // Test that the data is correct for each node.
  assert_node_data_is_eq(&arena, root, "root");
  assert_node_data_is_eq(&arena, child1, "child1");
  assert_node_data_is_eq(&arena, gc_1_id, "gc1");
  assert_node_data_is_eq(&arena, gc_2_id, "gc2");
  assert_node_data_is_eq(&arena, child_2_id, "child2");

  assert_eq!(arena.get_children_of(root).unwrap().len(), 2);
  assert_eq!(arena.get_parent_of(root).is_none(), true);

  assert_eq!(arena.get_children_of(child1).unwrap().len(), 2);
  assert_eq!(arena.get_parent_of(child1).unwrap(), root);

  // Test that tree walking works correctly for nodes.
  assert_eq!(arena.tree_walk_dfs(root).unwrap().len(), 5);

  let child1_and_descendants = arena.tree_walk_dfs(child1).unwrap();
  assert_eq!(child1_and_descendants.len(), 3);
  assert!(child1_and_descendants.contains(&child1));
  assert!(child1_and_descendants.contains(&gc_1_id));
  assert!(child1_and_descendants.contains(&gc_2_id));

  assert_eq!(arena.tree_walk_dfs(child_2_id).unwrap().len(), 1);
  assert!(arena
    .tree_walk_dfs(child_2_id)
    .unwrap()
    .contains(&child_2_id));

  // Test that node deletion works correclty.
  {
    println!(
      "{} {:?}",
      style_primary("root -before- ==>"),
      arena.tree_walk_dfs(root).unwrap()
    );
    let deletion_list = arena.delete_node(child1);
    assert_eq!(deletion_list.as_ref().unwrap().len(), 3);
    assert!(deletion_list.as_ref().unwrap().contains(&gc_1_id));
    assert!(deletion_list.as_ref().unwrap().contains(&gc_2_id));
    assert!(deletion_list.as_ref().unwrap().contains(&child1));
    println!(
      "{} {:?}",
      style_prompt("root -after- <=="),
      arena.tree_walk_dfs(root).unwrap()
    );
    assert_eq!(dbg!(arena.tree_walk_dfs(root).unwrap()).len(), 2);
  }

  // Helper functions.
  fn assert_node_data_is_eq(
    arena: &Arena<String>,
    node_id: usize,
    expected_name: &str,
  ) {
    let child_ref = arena.get_node_arc(node_id).unwrap();
    assert_eq!(child_ref.read().unwrap().payload, expected_name.to_string());
  }
}

#[test]
fn test_can_search_nodes_in_tree_with_filter_lambda() {
  let mut arena = Arena::<String>::new();

  // root
  //   +- child1
  //   |    +- gc1
  //   |    +- gc2
  //   +- child2

  let root = arena.add_new_node("root".to_string(), None);
  let child1 = arena.add_new_node("child1".to_string(), Some(root));
  let _gc_1_id = arena.add_new_node("gc1".to_string(), Some(child1));
  let _gc_2_id = arena.add_new_node("gc2".to_string(), Some(child1));
  let _child_2_id = arena.add_new_node("child2".to_string(), Some(root));
  println!("{}, {:#?}", style_primary("arena"), &arena);
  println!(
    "{}, {:#?}",
    style_primary("root"),
    arena.get_node_arc(root)
  );

  // Search entire arena for root.get_id().
  {
    let filter_id = root;
    let result = arena.filter_all_nodes_by(&mut move |id, _node_ref| {
      if id == filter_id {
        true
      } else {
        false
      }
    });
    assert_eq!(result.as_ref().unwrap().len(), 1);
  }

  // Search entire arena for node that contains payload "gc1".
  {
    let result = arena.filter_all_nodes_by(&mut move |_id, payload| {
      if payload == "gc1" {
        true
      } else {
        false
      }
    });
    assert_eq!(result.as_ref().unwrap().len(), 1);
  }
}

#[test]
fn test_mt_arena_insert_and_walk_in_parallel() {
  type ThreadResult = Vec<usize>;
  type Handles = Vec<JoinHandle<ThreadResult>>;

  let mut handles: Handles = Vec::new();
  let arena = MTArena::<String>::new();

  // Thread 1 - add root. Spawn and wait (since the 2 threads below need the root).
  {
    let arena_arc = arena.get_arena_arc();
    let thread = thread::spawn(move || {
      let mut arena_write = arena_arc.write().unwrap();
      let root = arena_write.add_new_node("foo".to_string(), None);
      vec![root]
    });
    thread.join().unwrap();
  }

  // Thread 2 - add child. Just spawn, don't wait to finish.
  {
    let arena_arc = arena.get_arena_arc();
    let thread = thread::spawn(move || {
      let mut arena_write = arena_arc.write().unwrap();
      let parent: Option<Vec<usize>> =
        arena_write.filter_all_nodes_by(&mut move |_id, payload| {
          if payload == "foo" {
            true
          } else {
            false
          }
        });
      let parent_id = parent.unwrap().first().unwrap().clone();
      let child = arena_write.add_new_node("bar".to_string(), Some(parent_id));
      vec![parent_id, child]
    });

    handles.push(thread);
  }

  // Thread 3 - add another child. Just spawn, don't wait to finish.
  {
    let arena_arc = arena.get_arena_arc();
    let thread = thread::spawn(move || {
      let mut arena_write = arena_arc.write().unwrap();
      let parent: Option<Vec<usize>> =
        arena_write.filter_all_nodes_by(&mut move |_id, payload| {
          if payload == "foo" {
            true
          } else {
            false
          }
        });
      let parent_id = parent.unwrap().first().unwrap().clone();
      let child = arena_write.add_new_node("baz".to_string(), Some(parent_id));
      vec![parent_id, child]
    });

    handles.push(thread);
  }

  // Wait for all threads to complete.
  handles.into_iter().for_each(move |handle| {
    handle.join().unwrap();
  });
  println!("{:#?}", &arena);

  // Perform tree walking in parallel. Note the lamda does capture many enclosing variable context.
  {
    let arena_arc = arena.get_arena_arc();
    let fn_arc = Arc::new(move |uid, payload| {
      println!(
        "{} {} {} Arena weak_count:{} strong_count:{}",
        style_primary("walker_fn - closure"),
        uid,
        payload,
        Arc::weak_count(&arena_arc),
        Arc::weak_count(&arena_arc)
      );
    });

    // Walk tree w/ a new thread using arc to lambda.
    {
      let thread_handle: JoinHandle<ResultUidList> =
        arena.tree_walk_parallel(0, fn_arc.clone());

      let result_node_list = thread_handle.join().unwrap();
      println!("{:#?}", result_node_list);
    }

    // Walk tree w/ a new thread using arc to lambda.
    {
      let thread_handle: JoinHandle<ResultUidList> =
        arena.tree_walk_parallel(1, fn_arc.clone());

      let result_node_list = thread_handle.join().unwrap();
      println!("{:#?}", result_node_list);
    }
  }
}
