/*******************************************************************************
 * Copyright (c) 2020 Association Cénotélie (cenotelie.fr)
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Lesser General Public License as
 * published by the Free Software Foundation, either version 3
 * of the License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Lesser General Public License for more details.
 *
 * You should have received a copy of the GNU Lesser General
 * Public License along with this program.
 * If not, see <http://www.gnu.org/licenses/>.
 ******************************************************************************/

//! Module for LR automata

use crate::grammars::{
    Grammar, Rule, RuleChoice, SymbolRef, TerminalRef, TerminalSet, GENERATED_AXIOM
};
use crate::ParsingMethod;
use hime_redist::parsers::{LRActionCode, LR_ACTION_CODE_REDUCE, LR_ACTION_CODE_SHIFT};
use std::collections::HashMap;

/// A reference to a grammar rule
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct RuleRef {
    /// The identifier of the variable
    pub variable: usize,
    /// The index of the rule for the variable
    pub index: usize
}

impl RuleRef {
    /// Creates a new rule reference
    pub fn new(variable: usize, index: usize) -> RuleRef {
        RuleRef { variable, index }
    }

    /// Gets the referenced rule in the grammar
    pub fn get_rule_in<'s, 'g>(&'s self, grammar: &'g Grammar) -> &'g Rule {
        &grammar
            .variables
            .iter()
            .find(|v| v.id == self.variable)
            .unwrap()
            .rules[self.index]
    }
}

/// The lookahead mode for LR items
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LookaheadMode {
    /// LR(0) item (no lookahead)
    LR0,
    /// LR(1) item (exactly one lookahead)
    LR1,
    /// LALR(1) item (multiple lookahead)
    LALR1
}

/// Represents a base LR item
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// The grammar rule for the item
    pub rule: RuleRef,
    /// The position in the grammar rule
    pub position: usize,
    /// The lookaheads for this item
    pub lookaheads: TerminalSet
}

impl Item {
    /// Gets the action for this item
    pub fn get_action(&self, grammar: &Grammar) -> LRActionCode {
        let rule = self.rule.get_rule_in(grammar);
        if self.position >= rule.body.choices[0].parts.len() {
            LR_ACTION_CODE_REDUCE
        } else {
            LR_ACTION_CODE_SHIFT
        }
    }

    /// Gets the symbol following the dot in this item
    pub fn get_next_symbol(&self, grammar: &Grammar) -> Option<SymbolRef> {
        let rule = self.rule.get_rule_in(grammar);
        if self.position >= rule.body.choices[0].parts.len() {
            None
        } else {
            Some(rule.body.choices[0].parts[self.position].symbol)
        }
    }

    /// Gets rule choice following the dot in this item
    pub fn get_next_choice<'s, 'g>(&'s self, grammar: &'g Grammar) -> Option<&'g RuleChoice> {
        let rule = self.rule.get_rule_in(grammar);
        if self.position >= rule.body.choices[0].parts.len() {
            None
        } else {
            Some(&rule.body.choices[self.position + 1])
        }
    }

    /// Gets the child of this item
    /// The child item is undefined if the action is REDUCE
    pub fn get_child(&self) -> Item {
        Item {
            rule: self.rule,
            position: self.position + 1,
            lookaheads: self.lookaheads.clone()
        }
    }

    /// Gets the context opened by this item
    pub fn get_opened_context(&self, grammar: &Grammar) -> Option<usize> {
        if self.position > 0 {
            // not at the beginning
            return None;
        }
        let rule = self.rule.get_rule_in(grammar);
        if self.position < rule.body.choices[0].parts.len() && rule.context != 0 {
            // this is a shift to a symbol with a context
            Some(rule.context)
        } else {
            None
        }
    }

    /// Closes this item into the given closure
    pub fn close_to(&self, grammar: &Grammar, closure: &mut Vec<Item>, mode: LookaheadMode) {
        if let Some(SymbolRef::Variable(sid)) = self.get_next_symbol(grammar) {
            // Here the item is of the form [Var -> alpha . next beta]
            // next is a variable
            // Firsts is a copy of the Firsts set for beta (next choice)
            // Firsts will contains symbols that may follow Next
            // Firsts will therefore be the lookahead for child items
            let mut firsts = self.get_next_choice(grammar).unwrap().firsts.clone();
            // If beta is nullifiable (contains ε) :
            if let Some(eps_index) = firsts
                .content
                .iter()
                .position(|x| *x == TerminalRef::Epsilon)
            {
                // Remove ε
                firsts.content.remove(eps_index);
                // Add the item's lookaheads
                firsts.add_others(&self.lookaheads);
            }
            let variable = grammar.get_variable(sid).unwrap();
            // For each rule that has Next as a head variable :
            for index in 0..variable.rules.len() {
                match mode {
                    LookaheadMode::LR0 => {
                        let candidate = Item {
                            rule: RuleRef::new(sid, index),
                            position: 0,
                            lookaheads: firsts.clone()
                        };
                        if !closure.contains(&candidate) {
                            closure.push(candidate);
                        }
                    }
                    LookaheadMode::LR1 => {
                        for terminal in firsts.clone().content.into_iter() {
                            let candidate = Item {
                                rule: RuleRef::new(sid, index),
                                position: 0,
                                lookaheads: TerminalSet::single(terminal)
                            };
                            if !closure.contains(&candidate) {
                                closure.push(candidate);
                            }
                        }
                    }
                    LookaheadMode::LALR1 => {
                        let candidate = Item {
                            rule: RuleRef::new(sid, index),
                            position: 0,
                            lookaheads: firsts.clone()
                        };
                        if let Some(other) =
                            closure.iter_mut().find(|item| item.same_base(&candidate))
                        {
                            other.lookaheads.add_others(&candidate.lookaheads);
                        } else {
                            closure.push(candidate);
                        }
                    }
                }
            }
        }
    }

    /// Gets whether the two items have the same base
    pub fn same_base(&self, other: &Item) -> bool {
        self.rule == other.rule && self.position == other.position
    }
}

/// Represents the kernel of a LR state
#[derive(Debug, Clone, Eq, Default)]
pub struct StateKernel {
    /// The items in this kernel
    pub items: Vec<Item>
}

impl PartialEq for StateKernel {
    fn eq(&self, other: &StateKernel) -> bool {
        self.items.len() == other.items.len()
            && self.items.iter().all(|item| other.items.contains(item))
    }
}

impl StateKernel {
    /// Gets the closure of this kernel
    pub fn into_state(self, grammar: &Grammar, mode: LookaheadMode) -> State {
        let mut items = self.items.clone();
        let mut i = 0;
        while i < items.len() {
            items[i].clone().close_to(grammar, &mut items, mode);
            i += 1;
        }
        State {
            kernel: self,
            items: items,
            children: HashMap::new(),
            opening_contexts: HashMap::new(),
            reductions: Vec::new()
        }
    }

    /// Adds an item to the kernel
    pub fn add_item(&mut self, item: Item) {
        if !self.items.contains(&item) {
            self.items.push(item);
        }
    }
}

/// Represents a reduction action in a LR state
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Reduction {
    /// The lookahead to reduce on
    pub lookahead: TerminalRef,
    /// The rule to reduce with
    pub rule: RuleRef,
    /// The length of the reduction for RNGLR parsers
    pub length: usize
}

/// Represents a LR state
#[derive(Debug, Clone)]
pub struct State {
    /// The state's kernel
    pub kernel: StateKernel,
    /// The state's item
    pub items: Vec<Item>,
    /// The state's children (transitions)
    pub children: HashMap<SymbolRef, usize>,
    /// The contexts opening by transitions from this state
    pub opening_contexts: HashMap<TerminalRef, Vec<usize>>,
    /// The reductions on this state
    pub reductions: Vec<Reduction>
}

impl State {
    /// Builds reductions for this state
    pub fn build_reductions_lr0(&mut self, id: usize, grammar: &Grammar) -> Conflicts {
        let mut conflicts = Conflicts::default();
        let mut reduce_index = None;
        for (index, item) in self.items.iter().enumerate() {
            if item.get_action(grammar) != LR_ACTION_CODE_REDUCE {
                continue;
            }
            if !self.children.is_empty() {
                // shift/reduce conflict
                conflicts.raise_shift_reduce(
                    self,
                    id,
                    grammar,
                    item.clone(),
                    TerminalRef::NullTerminal
                );
            }
            if let Some(previous_index) = reduce_index {
                // reduce/reduce conflict
                let previous: &Item = &self.items[previous_index];
                conflicts.raise_reduce_reduce(
                    id,
                    previous.clone(),
                    item.clone(),
                    TerminalRef::NullTerminal
                );
            } else {
                reduce_index = Some(index);
                self.reductions.push(Reduction {
                    lookahead: TerminalRef::NullTerminal,
                    rule: item.rule,
                    length: item.position
                });
            }
        }
        conflicts
    }

    /// Builds reductions for this state
    pub fn build_reductions_lr1(&mut self, id: usize, grammar: &Grammar) -> Conflicts {
        let mut conflicts = Conflicts::default();
        let mut reductions: HashMap<TerminalRef, usize> = HashMap::new();
        for (index, item) in self.items.iter().enumerate() {
            if item.get_action(grammar) != LR_ACTION_CODE_REDUCE {
                continue;
            }
            for lookahead in item.lookaheads.content.iter() {
                let symbol_ref: SymbolRef = (*lookahead).into();
                if self.children.contains_key(&symbol_ref) {
                    // There is already a shift action for the lookahead => conflict
                    conflicts.raise_shift_reduce(self, id, grammar, item.clone(), *lookahead);
                } else if let Some(previous_index) = reductions.get(lookahead) {
                    // There is already a reduction action for the lookahead => conflict
                    let previous: &Item = &self.items[*previous_index];
                    conflicts.raise_reduce_reduce(id, previous.clone(), item.clone(), *lookahead);
                } else {
                    // no conflict
                    reductions.insert(*lookahead, index);
                    self.reductions.push(Reduction {
                        lookahead: *lookahead,
                        rule: item.rule,
                        length: item.position
                    });
                }
            }
        }
        conflicts
    }

    /// Builds reductions for this state
    pub fn build_reductions_rnglr1(&mut self, id: usize, grammar: &Grammar) -> Conflicts {
        let mut conflicts = Conflicts::default();
        let mut reductions: HashMap<TerminalRef, usize> = HashMap::new();
        for (index, item) in self.items.iter().enumerate() {
            let rule = item.rule.get_rule_in(grammar);
            if item.get_action(grammar) == LR_ACTION_CODE_SHIFT
                && !rule.body.choices[item.position]
                    .firsts
                    .content
                    .contains(&TerminalRef::Epsilon)
            {
                // item is shift action and is not nullable after the dot
                continue;
            }
            for lookahead in item.lookaheads.content.iter() {
                let symbol_ref: SymbolRef = (*lookahead).into();
                if self.children.contains_key(&symbol_ref) {
                    // There is already a shift action for the lookahead => conflict
                    conflicts.raise_shift_reduce(self, id, grammar, item.clone(), *lookahead);
                } else if let Some(previous_index) = reductions.get(lookahead) {
                    // There is already a reduction action for the lookahead => conflict
                    let previous: &Item = &self.items[*previous_index];
                    conflicts.raise_reduce_reduce(id, previous.clone(), item.clone(), *lookahead);
                } else {
                    // no conflict
                    reductions.insert(*lookahead, index);
                    self.reductions.push(Reduction {
                        lookahead: *lookahead,
                        rule: item.rule,
                        length: item.position
                    });
                }
            }
        }
        conflicts
    }
}

/// Represents a LR graph
#[derive(Debug, Clone, Default)]
pub struct Graph {
    /// The states in this graph
    pub states: Vec<State>
}

impl Graph {
    /// Initializes a graph from the given state
    pub fn from(state: State, grammar: &Grammar, mode: LookaheadMode) -> Graph {
        let mut graph = Graph::default();
        graph.states.push(state);
        let mut i = 0;
        while i < graph.states.len() {
            graph.build_at_state(grammar, i, mode);
            i += 1;
        }
        graph
    }

    /// Build this graph at the given state
    fn build_at_state(&mut self, grammar: &Grammar, state_id: usize, mode: LookaheadMode) {
        // Shift dictionnary for the current set
        let mut shifts: HashMap<SymbolRef, StateKernel> = HashMap::new();
        // Build the children kernels from the shift actions
        for item in self.states[state_id].items.iter() {
            if let Some(next) = item.get_next_symbol(grammar) {
                shifts
                    .entry(next)
                    .or_insert_with(StateKernel::default)
                    .add_item(item.get_child());
            }
        }
        // Close the children and add them to the graph
        for (next, kernel) in shifts.into_iter() {
            let child_index = match self.get_state_for(&kernel) {
                Some(child_index) => child_index,
                None => self.add_state(kernel.into_state(grammar, mode))
            };
            self.states[state_id].children.insert(next, child_index);
        }
        // Build the context data
        let state = &mut self.states[state_id];
        for item in state.items.iter() {
            if let Some(context) = item.get_opened_context(grammar) {
                let mut opening_terminals = TerminalSet::default();
                match item.get_next_symbol(grammar) {
                    Some(SymbolRef::Virtual(sid)) => {
                        let variable = &grammar.get_variable(sid).unwrap();
                        opening_terminals.add_others(&variable.firsts);
                    }
                    Some(SymbolRef::Epsilon) => {
                        opening_terminals.add(TerminalRef::Epsilon);
                    }
                    Some(SymbolRef::Dollar) => {
                        opening_terminals.add(TerminalRef::Dollar);
                    }
                    Some(SymbolRef::Dummy) => {
                        opening_terminals.add(TerminalRef::Dummy);
                    }
                    Some(SymbolRef::NullTerminal) => {
                        opening_terminals.add(TerminalRef::NullTerminal);
                    }
                    Some(SymbolRef::Terminal(sid)) => {
                        opening_terminals.add(TerminalRef::Terminal(sid));
                    }
                    _ => {}
                }
                for terminal in opening_terminals.content.into_iter() {
                    let contexts = state.opening_contexts.entry(terminal).or_default();
                    if !contexts.contains(&context) {
                        contexts.push(context);
                    }
                }
            }
        }
    }

    /// Determines whether the given state (as a kernel) is already in this graph
    pub fn get_state_for(&self, kernel: &StateKernel) -> Option<usize> {
        self.states.iter().position(|state| &state.kernel == kernel)
    }

    /// Adds a state to this graph
    pub fn add_state(&mut self, state: State) -> usize {
        let index = self.states.len();
        self.states.push(state);
        index
    }

    /// Builds the reductions for this graph
    pub fn build_reductions_lr0(&mut self, grammar: &Grammar) -> Conflicts {
        let mut conflicts = Conflicts::default();
        for (index, state) in self.states.iter_mut().enumerate() {
            conflicts.aggregate(state.build_reductions_lr0(index, grammar));
        }
        conflicts
    }

    /// Builds the reductions for this graph
    pub fn build_reductions_lr1(&mut self, grammar: &Grammar) -> Conflicts {
        let mut conflicts = Conflicts::default();
        for (index, state) in self.states.iter_mut().enumerate() {
            conflicts.aggregate(state.build_reductions_lr1(index, grammar));
        }
        conflicts
    }

    /// Builds the reductions for this graph
    pub fn build_reductions_rnglr1(&mut self, grammar: &Grammar) -> Conflicts {
        let mut conflicts = Conflicts::default();
        for (index, state) in self.states.iter_mut().enumerate() {
            conflicts.aggregate(state.build_reductions_rnglr1(index, grammar));
        }
        conflicts
    }
}

/// Represents a phrase that can be produced by grammar.
/// It is essentially a list of terminals
#[derive(Debug, Clone, Eq)]
pub struct Phrase(Vec<TerminalRef>);

impl PartialEq for Phrase {
    fn eq(&self, other: &Phrase) -> bool {
        self.0.len() == other.0.len() && self.0.iter().zip(other.0.iter()).all(|(x, y)| x == y)
    }
}

impl Phrase {
    /// Appends a terminal to this phrase
    pub fn append(&mut self, terminal: TerminalRef) {
        self.0.push(terminal);
    }
}

/// The kinds of LR conflicts
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ConflictKind {
    /// Conflict between a shift action and a reduce action
    ShiftReduce,
    /// Conflict between two reduce actions
    ReduceReduce
}

/// A conflict between items
#[derive(Debug, Clone, Eq)]
pub struct Conflict {
    /// The state raising the conflict
    pub state: usize,
    /// The kind of conflict
    pub kind: ConflictKind,
    /// The items in the conflict
    pub items: Vec<Item>,
    /// The terminal that poses the conflict
    pub lookahead: TerminalRef
}

impl PartialEq for Conflict {
    fn eq(&self, other: &Conflict) -> bool {
        self.state == other.state && self.kind == other.kind && self.lookahead == other.lookahead
    }
}

/// A set of conflicts
#[derive(Debug, Default, Clone)]
pub struct Conflicts(Vec<Conflict>);

impl Conflicts {
    /// Raise a shift/reduce conflict
    pub fn raise_shift_reduce(
        &mut self,
        state: &State,
        state_id: usize,
        grammar: &Grammar,
        reducing: Item,
        lookahead: TerminalRef
    ) {
        // look for previous conflict
        for previous in self.0.iter_mut() {
            if previous.kind == ConflictKind::ShiftReduce && previous.lookahead == lookahead {
                // Previous conflict
                previous.items.push(reducing);
                return;
            }
        }
        // No previous conflict was found
        let mut items: Vec<Item> = state
            .items
            .iter()
            .filter(|item| item.get_next_symbol(grammar) == Some(lookahead.into()))
            .map(|item| item.clone())
            .collect();
        items.push(reducing);
        self.0.push(Conflict {
            state: state_id,
            kind: ConflictKind::ShiftReduce,
            items,
            lookahead
        });
    }

    /// Raise a reduce/reduce conflict
    pub fn raise_reduce_reduce(
        &mut self,
        state_id: usize,
        previous: Item,
        reducing: Item,
        lookahead: TerminalRef
    ) {
        // look for previous conflict
        for previous in self.0.iter_mut() {
            if previous.kind == ConflictKind::ReduceReduce && previous.lookahead == lookahead {
                // Previous conflict
                previous.items.push(reducing);
                return;
            }
        }
        // No previous conflict was found
        self.0.push(Conflict {
            state: state_id,
            kind: ConflictKind::ReduceReduce,
            items: vec![previous, reducing],
            lookahead
        });
    }

    /// Aggregate other conflicts into this collection
    pub fn aggregate(&mut self, mut other: Conflicts) {
        self.0.append(&mut other.0);
    }
}

/// Gets the LR(0) graph
fn get_graph_lr0(grammar: &Grammar) -> Graph {
    // Create the base LR(0) graph
    let axiom = grammar.get_variable_for_name(GENERATED_AXIOM).unwrap();
    let item = Item {
        rule: RuleRef::new(axiom.id, 0),
        position: 0,
        lookaheads: TerminalSet::default()
    };
    let kernel = StateKernel { items: vec![item] };
    let state0 = kernel.into_state(grammar, LookaheadMode::LR0);
    Graph::from(state0, grammar, LookaheadMode::LR0)
}

/// Builds a LR(0) graph
pub fn build_graph_lr0(grammar: &Grammar) -> (Graph, Conflicts) {
    let mut graph = get_graph_lr0(grammar);
    let conflicts = graph.build_reductions_lr0(grammar);
    (graph, conflicts)
}

/// Gets the LR(1) graph
fn get_graph_lr1(grammar: &Grammar) -> Graph {
    // Create the base LR(0) graph
    let axiom = grammar.get_variable_for_name(GENERATED_AXIOM).unwrap();
    let item = Item {
        rule: RuleRef::new(axiom.id, 0),
        position: 0,
        lookaheads: TerminalSet::default()
    };
    let kernel = StateKernel { items: vec![item] };
    let state0 = kernel.into_state(grammar, LookaheadMode::LR1);
    Graph::from(state0, grammar, LookaheadMode::LR1)
}

/// Builds a LR(1) graph
pub fn build_graph_lr1(grammar: &Grammar) -> (Graph, Conflicts) {
    let mut graph = get_graph_lr1(grammar);
    let conflicts = graph.build_reductions_lr1(grammar);
    (graph, conflicts)
}

/// Builds a RNGLR(1) graph
pub fn build_graph_rnglr1(grammar: &Grammar) -> (Graph, Conflicts) {
    let mut graph = get_graph_lr1(grammar);
    let conflicts = graph.build_reductions_rnglr1(grammar);
    (graph, conflicts)
}

/// Builds the kernels for a LALR(1) graph
fn build_graph_lalr1_kernels(graph0: &Graph) -> Vec<StateKernel> {
    // copy kernel without the lookaheads
    let mut kernels: Vec<StateKernel> = graph0
        .states
        .iter()
        .map(|state| StateKernel {
            items: state
                .kernel
                .items
                .iter()
                .map(|item| Item {
                    rule: item.rule,
                    position: item.position,
                    lookaheads: TerminalSet::default()
                })
                .collect()
        })
        .collect();
    // set epsilon as lookahead on all items in kernel 0
    for item in kernels[0].items.iter_mut() {
        item.lookaheads.add(TerminalRef::Epsilon);
    }
    kernels
}

/// Item in a propagation table
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Propagation {
    from_state: usize,
    from_item: usize,
    to_state: usize,
    to_item: usize
}

/// Builds the propagation table for a LALR(1) graph
fn build_graph_lalr1_propagation_table(
    graph0: &Graph,
    grammar: &Grammar,
    kernels: &mut Vec<StateKernel>
) -> Vec<Propagation> {
    let mut propagation = Vec::new();
    for i in 0..kernels.len() {
        // For each LALR(1) item in the kernel
        // Only the kernel needs to be examined as the other items will be discovered and treated
        // with the dummy closures
        for item_id in 0..(kernels[i].items.len()) {
            if kernels[i].items[item_id].get_action(grammar) == LR_ACTION_CODE_REDUCE {
                // If item is of the form [A -> alpha .]
                // => The closure will only contain the item itself
                // => Cannot be used to generate or propagate lookaheads
                continue;
            }
            // Item here is of the form [A -> alpha . beta]
            // Create the corresponding dummy item : [A -> alpha . beta, dummy]
            // This item is used to detect lookahead propagation
            let dummy_state = StateKernel {
                items: vec![Item {
                    rule: kernels[i].items[item_id].rule,
                    position: kernels[i].items[item_id].position,
                    lookaheads: TerminalSet::single(TerminalRef::Dummy)
                }]
            }
            .into_state(grammar, LookaheadMode::LALR1);
            // For each item in the closure of the dummy item
            for dummy_item in dummy_state.items.iter() {
                if let Some(next_symbol) = dummy_item.get_next_symbol(grammar) {
                    // not a reduction
                    let dummy_child = dummy_item.get_child();
                    // Get the child item in the child LALR(1) kernel
                    let child_state = *graph0.states[i].children.get(&next_symbol).unwrap();
                    let child_item = kernels[child_state]
                        .items
                        .iter()
                        .position(|candidate| candidate.same_base(&dummy_child))
                        .unwrap();
                    // If the lookaheads of the item in the dummy set contains the dummy terminal
                    if dummy_item.lookaheads.content.contains(&TerminalRef::Dummy) {
                        // => Propagation from the parent item to the child
                        propagation.push(Propagation {
                            from_state: i,
                            from_item: item_id,
                            to_state: child_state,
                            to_item: child_item
                        });
                    } else {
                        // => Spontaneous generation of lookaheads
                        for lookahead in dummy_item.lookaheads.content.iter() {
                            kernels[child_state].items[child_item]
                                .lookaheads
                                .add(*lookahead);
                        }
                    }
                }
            }
        }
    }
    propagation
}

/// Executes the propagation for a LALR(1) graph
fn build_graph_lalr1_propagate(kernels: &mut Vec<StateKernel>, table: &Vec<Propagation>) {
    let mut modifications = 1;
    while modifications != 0 {
        modifications = 0;
        for propagation in table.iter() {
            let before = kernels[propagation.to_state].items[propagation.to_item]
                .lookaheads
                .content
                .len();
            let others = kernels[propagation.from_state].items[propagation.from_item]
                .lookaheads
                .clone();
            kernels[propagation.to_state].items[propagation.to_item]
                .lookaheads
                .add_others(&others);
            let after = kernels[propagation.to_state].items[propagation.to_item]
                .lookaheads
                .content
                .len();
            modifications += after - before;
        }
    }
}

/// Builds the complete LALR(1) graph
fn build_graph_lalr1_graph(kernels: Vec<StateKernel>, graph0: &Graph, grammar: &Grammar) -> Graph {
    // Build states
    let mut states: Vec<State> = kernels
        .into_iter()
        .map(|kernel| kernel.into_state(grammar, LookaheadMode::LALR1))
        .collect();
    // Link for each LALR(1) set
    for (state0, state1) in graph0.states.iter().zip(states.iter_mut()) {
        state1.children = state0.children.clone();
        state1.opening_contexts = state0.opening_contexts.clone();
    }
    Graph { states }
}

/// Gets the LALR(1) graph
fn get_graph_lalr1(grammar: &Grammar) -> Graph {
    let graph0 = get_graph_lr0(grammar);
    let mut kernels = build_graph_lalr1_kernels(&graph0);
    let propagation = build_graph_lalr1_propagation_table(&graph0, grammar, &mut kernels);
    build_graph_lalr1_propagate(&mut kernels, &propagation);
    build_graph_lalr1_graph(kernels, &graph0, grammar)
}

/// Builds a LALR(1) graph
pub fn build_graph_lalr1(grammar: &Grammar) -> (Graph, Conflicts) {
    let mut graph = get_graph_lalr1(grammar);
    let conflicts = graph.build_reductions_lr1(grammar);
    (graph, conflicts)
}

/// Builds a RNGLALR(1) graph
pub fn build_graph_rnglalr1(grammar: &Grammar) -> (Graph, Conflicts) {
    let mut graph = get_graph_lalr1(grammar);
    let conflicts = graph.build_reductions_rnglr1(grammar);
    (graph, conflicts)
}

/// Build the specified grammar
pub fn build_graph(grammar: &Grammar, method: ParsingMethod) -> (Graph, Conflicts) {
    match method {
        ParsingMethod::LR0 => build_graph_lr0(grammar),
        ParsingMethod::LR1 => build_graph_lr1(grammar),
        ParsingMethod::LALR1 => build_graph_lalr1(grammar),
        ParsingMethod::RNGLR1 => build_graph_rnglr1(grammar),
        ParsingMethod::RNGLALR1 => build_graph_rnglalr1(grammar)
    }
}
