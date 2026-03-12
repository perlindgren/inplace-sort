import Mathlib.Data.Fintype.Basic
import Mathlib.Data.Option.Basic
-- import Mathlib.Data.Nat.Basic
import Mathlib.Order.Basic
import Mathlib.Data.Finset.Basic
import Mathlib.Data.Finset.Card
import Mathlib.Data.List.MinMax

open Set

-- === Reachability ===

-- iterate next k times
def iterateOptionNext (next : N → Option N) : Option N → ℕ → Option N
| none, _ => none
| some n, 0 => some n
| some n, Nat.succ k => iterateOptionNext next (next n) k
termination_by _ k => k

-- reachable nodes {→*}
def reachableStar (fnext : N → Option N) (X : Option N) : Set N :=
{ n : N | ∃ k : ℕ, iterateOptionNext fnext X k = some n }

-- reachable nodes {→+}
def reachablePlus (next : N → Option N) (X : Option N) : Set N :=
match X with
| none   => ∅
| some x => {n | ∃ k : Nat, k ≥ 1 ∧ iterateOptionNext next (some x) k = some n}

section List

-- list structure
structure InplaceList (N V : Type) where
  H : Option N
  T : Option N
  F : Option N
  next : N → Option N
  data : N → Option V
  Min : Option V
  Cur : Option (N × V × Option V × Option N)

-- list operations

-- insert a value into the list
def insertVal {N V : Type} [DecidableEq N] [LinearOrder V]
(h : InplaceList N V) (v : V) : InplaceList N V :=
{ H := match h.H with
    | some _ => h.F
    | none => h.H,
  T := match h.F with
    | some _ => h.F
    | none        => h.T,
  F := match h.F with
    | some f_node => h.next f_node
    | none        => h.F
  next := match h.F with -- must have free nodes to insert
    | some _ => match h.H with
                  | some _ => fun n => if some n = h.T then h.F
                                       else if n = h.F then none else h.next n
                  | none => fun n => if some n = h.F then none else h.next n
    | none => h.next,
  data := match h.F with
    | some _ => fun n => if some n = h.F then some v else h.data n
    | none => h.data,
  Min := match h.F with
    | some _ => match h.Min with
                  | some old_min => some (min old_min v)
                  | none   => some v
    | none => h.Min,
  Cur := h.Cur }

-- forward cursor (TODO)
def forwardCursor {N V : Type} [DecidableEq N] [LinearOrder V]
(h : InplaceList N V) : InplaceList N V :=
{ H := h.H,
  T := h.T,
  F := h.F,
  next := h.next,
  data := h.data,
  Min := h.Min,
  Cur := h.Cur }

-- extract minimum from the list (TODO)
def extractMin {N V : Type} [DecidableEq N] [LinearOrder V]
(h : InplaceList N V) : InplaceList N V :=
{ H := h.H,
  T := h.T,
  F := h.F,
  next := h.next,
  data := h.data,
  Min := h.Min,
  Cur := h.Cur }

end List


-- min of a finite multiset
noncomputable def minOfMultiset (X : Multiset V) [LinearOrder V] : Option V :=
X.toFinset.val.toList.minimum

-- second smallest item (min2) of a finite multiset
noncomputable def min2OfMultiset (X : Multiset V) [LinearOrder V] : Option V :=
match minOfMultiset X with
| none => none
| some m =>
    let X' := X.erase m
    minOfMultiset X'

def nodesAfter (next : N → Option N) (C : N) : Set N :=
reachablePlus next (some C)

def nodesBefore (next : N → Option N) (H : N) (C : N) : Set N :=
reachableStar next H \ nodesAfter next C

open Classical in
noncomputable def setToFinset {N : Type} [Fintype N] [DecidableEq N]
  (s : Set N) : Finset N :=
Finset.univ.filter s

noncomputable def valuesOfSet
  {N V : Type}
  [Fintype N] [DecidableEq N]
  (X : Set N) [DecidablePred X]
  (data : N → Option V) :
  Multiset V :=
((setToFinset X).val.filterMap data)


-- predicates

-- list predicate (no cycles)
def isList (next : N → Option N) (X : Option N) : Prop :=
∀ n ∈ reachableStar next X, n ∉ reachablePlus next (some n)

-- list of used and empty nodes cover all of N, and they do not overlap
def inv_nodes (h : InplaceList N V) : Prop :=
(Set.univ : Set N) =
  reachableStar h.next h.H ∪ reachableStar h.next h.F ∧
reachableStar h.next h.H ∩ reachableStar h.next h.F = ∅

-- the lists of used and empty nodes have no loops
def inv_no_loops (h : InplaceList N V) : Prop :=
(h.H ≠ none → isList h.next h.H) ∧
(h.F ≠ none → isList h.next h.F)

-- T points to the tail of the used nodes list
def inv_tail (h : InplaceList N V) : Prop :=
h.T ≠ none →
  (∃ t, h.T = some t ∧
        t ∈ reachableStar h.next h.H ∧
        h.next t = none)

def inv_min_basic (h : InplaceList N V) : Prop :=
(h.Min = none ↔ h.H = none)

open Classical in
noncomputable def inv_cursor
  {N V : Type}
  [Fintype N] [DecidableEq N] [LinearOrder V]
  (h : InplaceList N V) : Prop :=
match h.Cur, h.H with
| none, _ => True
| some _, none => False
| some (C, minVal, min2Val, prev), some H =>
  C ∈ reachableStar h.next h.H ∧
  let nodes :=
    setToFinset (nodesBefore h.next H C)
  let valuesBefore :=
    valuesOfSet nodes h.data
  minVal = minOfMultiset valuesBefore ∧
  min2Val = min2OfMultiset valuesBefore ∧
  match prev with
  | none => h.data H = some minVal
  | some prev_node => match h.next prev_node with
                      | none => False
                      | some next_node => h.data next_node = some minVal


def Invariant {N V : Type}
  [Fintype N] [DecidableEq N] [LinearOrder V] (h : InplaceList N V) : Prop :=
inv_nodes h ∧
inv_no_loops h ∧
inv_tail h ∧
inv_min_basic h ∧
inv_cursor h
