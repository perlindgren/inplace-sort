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

-- list predicate (no cycles)
def isList (next : N → Option N) (X : Option N) : Prop :=
∀ n ∈ reachableStar next X, n ∉ reachablePlus next (some n)

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
def insertVal {N V : Type} [Fintype N] [DecidableEq N] [LinearOrder V]
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
def forwardCursor {N V : Type} [Fintype N] [DecidableEq N] [LinearOrder V]
(h : InplaceList N V) : InplaceList N V :=
{ H := h.H,
  T := h.T,
  F := h.F,
  next := h.next,
  data := h.data,
  Min := h.Min,
  Cur := h.Cur }

-- extract minimum from the list (TODO)
def extractMin {N V : Type} [Fintype N] [DecidableEq N] [LinearOrder V]
(h : InplaceList N V) : InplaceList N V :=
{ H := h.H,
  T := h.T,
  F := h.F,
  next := h.next,
  data := h.data,
  Min := h.Min,
  Cur := h.Cur }

end List

-- define min and min2
section MinValues

-- min of a nonempty finite set
-- noncomputable def minOfFinset (X : Finset V) [LinearOrder V] [Fintype V] [Nonempty V] : Option V :=
-- X.val.toList.minimum

-- -- second smallest of a finite set
-- noncomputable def min2OfFinset (X: Finset V) [LinearOrder V] [Fintype V]: Option V
-- def min2OfFinset (X : Finset V) (h : X.nonempty) : Option V :=
-- if X.card = 1 then none else
-- let m := Finset.min' X h
-- let X' := X.erase m
-- if X'.nonempty then some (Finset.min' X' (Finset.card_pos.2 (by
--   simp [X', h])) )
-- else some m

end MinValues

def nodesAfter (h : InplaceList N V) (C : N) : Set N :=
reachablePlus h.next (some C)

def nodesBefore (h : InplaceList N V) (C : N) : Set N :=
reachableStar h.next h.H \ nodesAfter h C

-- -- invariants

-- -- -- list of used and empty nodes cover all of N, and they do not overlap
-- -- def inv_nodes (h : InplaceList N V) : Prop :=
-- -- (Set.univ : Set N) =
-- --   reachableStar h.next h.H ∪ reachableStar h.next h.F ∧
-- -- reachableStar h.next h.H ∩ reachableStar h.next h.F = ∅

-- -- -- the lists of used and empty nodes have no loops
-- -- def inv_no_loops (h : InplaceList N V) : Prop :=
-- -- (h.H ≠ none → isList h.next h.H) ∧
-- -- (h.F ≠ none → isList h.next h.F)

-- -- -- T points to the tail of the used nodes list
-- -- def inv_tail (h : InplaceList N V) : Prop :=
-- -- h.T ≠ none →
-- --   (∃ t, h.T = some t ∧
-- --         t ∈ reachableStar h.next h.H ∧
-- --         h.next t = none)

-- def inv_min_basic (h : InplaceList N V) : Prop :=
-- (h.Min = none ↔ h.H = none)


-- -- def inv_cursor (h : InplaceList N V) : Prop :=
-- -- match h.Cur with
-- -- | none => True
-- -- | some (C, minVal, min2Val, prev) =>
-- --     -- C ∈ {H→*}
-- --     C ∈ reachableStar h.next h.H ∧
-- --     -- nodes before C
-- --     let nodes := nodesBefore h C
-- --     let vals := dataValues h nodes
-- --     vals.nonempty ∧
-- --     -- min is smallest in vals
-- --     minVal = minOfFinset vals vals.nonempty ∧
-- --     -- min2 follows the rules
-- --     min2Val = min2OfFinset vals vals.nonempty ∧
-- --     -- prev points correctly (either none or to node with data = min)
-- --     (prev = none ∧ h.data (h.H.getD C) = some minVal ∨
-- --      ∃ p, prev = some p ∧ h.data p = some minVal)

-- -- def Invariant (h : InplaceList N V) : Prop :=
-- -- inv_nodes h ∧
-- -- inv_no_loops h ∧
-- -- inv_tail h ∧
-- -- inv_min_basic h --∧
-- --inv_cursor h
