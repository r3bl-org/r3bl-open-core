/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

//! Integration tests for the `tree_memory_arena` module.

use std::{collections::VecDeque,
          sync::Arc,
          thread::{self, JoinHandle}};

use r3bl_rs_utils::{tree_memory_arena::{Arena, MTArena, ResultUidList},
                    TraversalKind};
use r3bl_rs_utils_core::{assert_eq2, style_primary, style_prompt};

#[test]
fn test_can_add_nodes_to_tree() {
    // Can create an arena.
    let mut arena = Arena::<usize>::new();
    let node_1_value = 42_usize;
    let node_2_value = 100_usize;

    // Can insert a node - node_1.
    {
        let node_1_id = arena.add_new_node(node_1_value, None);
        assert_eq2!(node_1_id, 0);
    }

    // Can find node_1 by id.
    {
        let node_1_id = 0_usize;
        assert!(arena.get_node_arc(node_1_id).is_some());

        let node_1_ref = dbg!(arena.get_node_arc(node_1_id).unwrap());
        let node_1_ref_weak = arena.get_node_arc_weak(node_1_id).unwrap();
        assert_eq2!(node_1_ref.read().unwrap().payload, node_1_value);
        assert_eq2!(
            node_1_ref_weak.upgrade().unwrap().read().unwrap().payload,
            42
        );
    }

    // Mutate node_1.
    {
        let node_1_id = 0_usize;
        {
            let node_1_ref = dbg!(arena.get_node_arc(node_1_id).unwrap());
            node_1_ref.write().unwrap().payload = 100;
        }
        assert_eq2!(
            arena
                .get_node_arc(node_1_id)
                .unwrap()
                .read()
                .unwrap()
                .payload,
            100
        );
    }

    // Can't find node by id that doesn't exist.
    {
        let node_id_dne = 200_usize;
        assert!(arena.get_node_arc(node_id_dne).is_none());
    }

    // Can add child to node_1.
    {
        let node_1_id = 0_usize;
        let node_2_id = arena.add_new_node(node_2_value, node_1_id.into());
        let node_2_ref = dbg!(arena.get_node_arc(node_2_id).unwrap());
        let node_2_ref_weak = arena.get_node_arc_weak(node_2_id).unwrap();
        assert_eq2!(node_2_ref.read().unwrap().payload, node_2_value);
        assert_eq2!(
            node_2_ref_weak.upgrade().unwrap().read().unwrap().payload,
            node_2_value
        );
    }

    // Can dfs tree walk.
    {
        let node_1_id = 0_usize;
        let node_2_id = 1_usize;

        let node_list = dbg!(arena.tree_walk_dfs(node_1_id).unwrap());

        assert_eq2!(node_list.len(), 2);
        assert_eq2!(node_list, vec![node_1_id, node_2_id]);
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

    let root_id = arena.add_new_node("root".to_string(), None);
    let child1_id = arena.add_new_node("child1".to_string(), root_id.into());
    let gc1_id = arena.add_new_node("gc1".to_string(), child1_id.into());
    let gc2_id = arena.add_new_node("gc2".to_string(), child1_id.into());
    let child2_id = arena.add_new_node("child2".to_string(), root_id.into());
    println!("{}, {arena:#?}", style_primary("arena"));

    // Test that the data is correct for each node.
    assert_node_data_is_eq(&arena, root_id, "root");
    assert_node_data_is_eq(&arena, child1_id, "child1");
    assert_node_data_is_eq(&arena, gc1_id, "gc1");
    assert_node_data_is_eq(&arena, gc2_id, "gc2");
    assert_node_data_is_eq(&arena, child2_id, "child2");

    assert_eq2!(arena.get_children_of(root_id).unwrap().len(), 2);
    assert!(arena.get_parent_of(root_id).is_none());

    assert_eq2!(arena.get_children_of(child1_id).unwrap().len(), 2);
    assert_eq2!(arena.get_parent_of(child1_id).unwrap(), root_id);

    // Test that tree walking works correctly for nodes - DFS.
    {
        assert_eq2!(
            arena.tree_walk_dfs(root_id).unwrap(),
            VecDeque::from([root_id, child1_id, gc1_id, gc2_id, child2_id])
        );

        let child1_and_descendants = arena.tree_walk_dfs(child1_id).unwrap();
        assert_eq2!(child1_and_descendants, vec![child1_id, gc1_id, gc2_id]);
    }

    // Test that tree walking works correctly for nodes - BFS.
    {
        assert_eq2!(
            arena.tree_walk_bfs(root_id).unwrap(),
            VecDeque::from([root_id, child1_id, child2_id, gc1_id, gc2_id])
        );

        let child1_and_descendants = arena.tree_walk_bfs(child1_id).unwrap();
        assert_eq2!(child1_and_descendants, vec![child1_id, gc1_id, gc2_id]);
    }

    // Test that node deletion works correctly.
    {
        println!(
            "{} {:?}",
            style_primary("root -before- ==>"),
            arena.tree_walk_dfs(root_id).unwrap()
        );
        let deletion_list = arena.delete_node(child1_id);
        assert_eq2!(deletion_list.as_ref().unwrap().len(), 3);
        assert!(deletion_list.as_ref().unwrap().contains(&gc1_id));
        assert!(deletion_list.as_ref().unwrap().contains(&gc2_id));
        assert!(deletion_list.as_ref().unwrap().contains(&child1_id));
        println!(
            "{} {:?}",
            style_prompt("root -after- <=="),
            arena.tree_walk_dfs(root_id).unwrap()
        );
        assert_eq2!(dbg!(arena.tree_walk_dfs(root_id).unwrap()).len(), 2);
    }

    // Helper functions.
    fn assert_node_data_is_eq(
        arena: &Arena<String>,
        node_id: usize,
        expected_name: &str,
    ) {
        let child_ref = arena.get_node_arc(node_id).unwrap();
        assert_eq2!(child_ref.read().unwrap().payload, expected_name.to_string());
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
    let child1 = arena.add_new_node("child1".to_string(), root.into());
    let _gc_1_id = arena.add_new_node("gc1".to_string(), child1.into());
    let _gc_2_id = arena.add_new_node("gc2".to_string(), child1.into());
    let _child_2_id = arena.add_new_node("child2".to_string(), root.into());
    println!("{}, {:#?}", style_primary("arena"), &arena);
    println!("{}, {:#?}", style_primary("root"), arena.get_node_arc(root));

    // Search entire arena for root.get_id().
    {
        let filter_id = root;
        let result = arena.filter_all_nodes_by(&move |id, _node_ref| id == filter_id);
        assert_eq2!(result.as_ref().unwrap().len(), 1);
    }

    // Search entire arena for node that contains payload "gc1".
    {
        let result = arena.filter_all_nodes_by(&move |_id, payload| payload == "gc1");
        assert_eq2!(result.as_ref().unwrap().len(), 1);
    }
}

#[test]
fn test_mt_arena_insert_and_walk_in_parallel() {
    type ThreadResult = Vec<usize>;
    type Handles = Vec<JoinHandle<ThreadResult>>;

    let mut handles: Handles = Vec::new();
    let arena = MTArena::<String>::new();

    // Thread 1 - add root. Spawn and wait (since the 2 threads below need the
    // root).
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
            let parent: Option<VecDeque<usize>> =
                arena_write.filter_all_nodes_by(&move |_id, payload| payload == "foo");
            let parent_id = *parent.unwrap().front().unwrap();
            let child = arena_write.add_new_node("bar".to_string(), parent_id.into());
            vec![parent_id, child]
        });

        handles.push(thread);
    }

    // Thread 3 - add another child. Just spawn, don't wait to finish.
    {
        let arena_arc = arena.get_arena_arc();
        let thread = thread::spawn(move || {
            let mut arena_write = arena_arc.write().unwrap();
            let parent: Option<VecDeque<usize>> =
                arena_write.filter_all_nodes_by(&move |_id, payload| payload == "foo");
            let parent_id = *parent.unwrap().front().unwrap();
            let child = arena_write.add_new_node("baz".to_string(), parent_id.into());
            vec![parent_id, child]
        });

        handles.push(thread);
    }

    // Wait for all threads to complete.
    handles.into_iter().for_each(move |handle| {
        handle.join().unwrap();
    });
    println!("{:#?}", &arena);

    // Perform tree walking in parallel. Note the lambda does capture many enclosing
    // variable context.
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
                arena.tree_walk_parallel(0, fn_arc.clone(), TraversalKind::BreadthFirst);

            let result_node_list = thread_handle.join().unwrap();
            println!("{result_node_list:#?}");
        }

        // Walk tree w/ a new thread using arc to lambda.
        {
            let thread_handle: JoinHandle<ResultUidList> =
                arena.tree_walk_parallel(1, fn_arc, TraversalKind::DepthFirst);

            let result_node_list = thread_handle.join().unwrap();
            println!("{result_node_list:#?}");
        }
    }
}
