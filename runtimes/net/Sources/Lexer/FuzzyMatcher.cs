﻿/**********************************************************************
* Copyright (c) 2015 Laurent Wouters
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
*
* Contributors:
*     Laurent Wouters - lwouters@xowl.org
**********************************************************************/
using System.Collections.Generic;

namespace Hime.Redist.Lexer
{
	/// <summary>
	/// A fuzzy DFA matcher
	/// This matcher uses the Levenshtein distance to match the input ahead against the current DFA automaton.
	/// The matcher favors solutions that are the closest to the original input.
	/// When multiple solutions are at the same Levenshtein distance to the input, the longest one is preferred.
	/// </summary>
	class FuzzyMatcher
	{
		/// <summary>
		/// A DFA stack node
		/// </summary>
		private class Node
		{
			/// <summary>
			/// The previous node
			/// </summary>
			public readonly Node previous;
			/// <summary>
			/// The represented DFA state
			/// </summary>
			public readonly int state;
			/// <summary>
			/// The required length in the input to reach this node
			/// </summary>
			public readonly int length;
			/// <summary>
			/// The Levenshtein distance of this node from the input
			/// </summary>
			public readonly int distance;
			/// <summary>
			/// The raised lexical error to reach this state
			/// </summary>
			public readonly ParseError error;


			/// <summary>
			/// Initializes this node
			/// </summary>
			/// <param name="previous">The previous node</param>
			/// <param name="state">The represented DFA state</param>
			/// <param name="length">The required length in the input to reach this node</param>
			/// <param name="distance">The Levenshtein distance of this node from the input</param>
			public Node(Node previous, int state, int length, int distance)
			{
				this.previous = previous;
				this.state = state;
				this.length = length;
				this.distance = distance;
				error = null;
			}

			/// <summary>
			/// Initializes this node
			/// </summary>
			/// <param name="previous">The previous node</param>
			/// <param name="state">The represented DFA state</param>
			/// <param name="length">The required length in the input to reach this node</param>
			/// <param name="distance">The Levenshtein distance of this node from the input</param>
			/// <param name="error">The raised lexical error to reach this state</param>
			public Node(Node previous, int state, int length, int distance, ParseError error)
			{
				this.previous = previous;
				this.state = state;
				this.length = length;
				this.distance = distance;
				this.error = null;
				this.error = error;
			}
		}

		/// <summary>
		/// This lexer's automaton
		/// </summary>
		private readonly Automaton automaton;
		/// <summary>
		/// The input text
		/// </summary>
		private readonly BaseText text;
		/// <summary>
		/// Delegate for raising errors
		/// </summary>
		private readonly AddLexicalError errors;
		/// <summary>
		/// The maximum Levenshtein distance between the input and the DFA
		/// </summary>
		private readonly int maxDistance;
		/// <summary>
		/// The index in the input from wich the error was raised
		/// </summary>
		private readonly int originIndex;
		/// <summary>
		/// The queue of DFA stack heads to inspect
		/// </summary>
		private readonly List<Node> queue;
		/// <summary>
		/// The longest matching node
		/// </summary>
		private Node matching;

		/// <summary>
		/// Initializes this matcher
		/// </summary>
		/// <param name="automaton">This lexer's automaton</param>
		/// <param name="text">The input text</param>
		/// <param name="errors">Delegate for raising errors</param>
		/// <param name="maxDistance">The maximum Levenshtein distance between the input and the DFA</param>
		/// <param name="index">The index in the input from wich the error was raised</param>
		public FuzzyMatcher(Automaton automaton, BaseText text, AddLexicalError errors, int maxDistance, int index)
		{
			this.automaton = automaton;
			this.text = text;
			this.errors = errors;
			this.maxDistance = maxDistance;
			originIndex = index;
			queue = new List<Node>();
			matching = null;
		}

		/// <summary>
		/// Runs this matcher
		/// </summary>
		/// <returns>The solution</returns>
		public TokenMatch Run()
		{
			queue.Add(new Node(null, 0, 0, 0));
			for (int i = 0; i != queue.Count; i++)
				Inspect(queue[i]);
			return matching != null ? OnSuccess(matching) : OnFailure();
		}

		/// <summary>
		/// Constructs the solution when succeeded to fix the error
		/// </summary>
		/// <param name="node">The node with the solution</param>
		/// <returns>The constructed solution</returns>
		private TokenMatch OnSuccess(Node node)
		{
			List<ParseError> myErrors = new List<ParseError>();
			Node current = node;
			while (current != null)
			{
				if (current.error != null)
					myErrors.Add(current.error);
				current = current.previous;
			}
			for (int i = myErrors.Count - 1; i != -1; i--)
				errors(myErrors[i]);
			return new TokenMatch(node.state, node.length);
		}

		/// <summary>
		/// Constructs the solution when failed to fix the error
		/// </summary>
		private TokenMatch OnFailure()
		{
			errors(new UnexpectedCharError(text.GetValue(originIndex).ToString(), text.GetPositionAt(originIndex)));
			return new TokenMatch(1);
		}

		/// <summary>
		/// Inspects the current stack head
		/// </summary>
		/// <param name="head">The head of a DFA stack</param>
		private void Inspect(Node head)
		{
			// gather data for this node
			int index = originIndex + head.length;
			bool atEnd = text.IsEnd(index);
			char current = atEnd ? '\0' : text.GetValue(index);
			AutomatonState stateData = automaton.GetState(head.state);

			// is it a matching state
			if (stateData.TerminalsCount != 0)
			{
				// favor a match if
				// * this is the first one
				// * or, it is strictly closer to the original input
				// * or, it is at the same instance to the original input, but strictly matches more input
				if (matching == null || head.distance < matching.distance || (head.distance == matching.distance && head.length > matching.length))
					matching = head;
			}

			if (!atEnd && head.distance < maxDistance)
			{
				// not at end and not at the maximum distance, we can drop
				queue.Add(new Node(head,
					head.state,
					head.length + 1,
					head.distance + 1,
					new UnexpectedCharError(text.GetValue(index).ToString(), text.GetPositionAt(index))));
			}

			if (stateData.IsDeadEnd)
				return;

			for (int i = 0; i != 256; i++)
			{
				int target = stateData.GetCachedTransition(i);
				if (target == Automaton.DEAD_STATE)
					continue;
				if (current == i)
				{
					// this is the current input value
					Enqueue(head,
						target,
						head.length + 1,
						head.distance,
						null);
				}
				if (head.distance < maxDistance)
				{
					// not at the max distance
					if (!atEnd)
					{
						// not at the end
						// try to replace the next character by an expected one
						Enqueue(head,
							target,
							head.length + 1,
							head.distance + 1,
							new UnexpectedCharError(text.GetValue(index).ToString(), text.GetPositionAt(index)));

					}
					// try to insert the expected character
					Enqueue(head,
						target,
						head.length,
						head.distance + 1,
						new UnexpectedCharError(atEnd ? "" : text.GetValue(index).ToString(), text.GetPositionAt(index)));
				}
			}

			for (int i = 0; i != stateData.BulkTransitionsCount; i++)
			{
				AutomatonTransition transition = stateData.GetBulkTransition(i);
				if (current >= transition.Start && current <= transition.End)
				{
					// this is the current input value
					Enqueue(head,
						transition.Target,
						head.length + 1,
						head.distance,
						null);
				}
				if (head.distance < maxDistance)
				{
					// not at the max distance
					if (!atEnd)
					{
						// not at the end
						// try to replace the next character by an expected one
						Enqueue(head,
							transition.Target,
							head.length + 1,
							head.distance + 1,
							new UnexpectedCharError(text.GetValue(index).ToString(), text.GetPositionAt(index)));

					}
					// try to insert the expected character
					Enqueue(head,
						transition.Target,
						head.length,
						head.distance + 1,
						new UnexpectedCharError(atEnd ? "" : text.GetValue(index).ToString(), text.GetPositionAt(index)));
				}
			}
		}

		/// <summary>
		/// Enqueues a new head if it is of interest.
		/// A new head is of interest if:
		/// * This is the first time the DFA state has been reached
		/// * Or, for the same state, it is a strictly longer input match
		/// * Or, for the same state, and the same length, the Levenshtein distance is strictly less
		/// </summary>
		/// <param name="previous">The parent head</param>
		/// <param name="state">The target DFA state</param>
		/// <param name="length">The matched input length</param>
		/// <param name="distance">The Levenshtein distance between the matched input and the DFA</param>
		/// <param name="error">The raised error, if any</param>
		private void Enqueue(Node previous, int state, int length, int distance, ParseError error)
		{
			for (int i = queue.Count - 1; i != -1; i--)
			{
				if (queue[i].state != state)
					// not the same state, could be of interest
					continue;
				// this is the same DFA state
				// the length if strictly less that the one already enqueued, not of interest
				if (length < queue[i].length)
					return;
				if (length > queue[i].length)
					// strictly longer, could be of interest
					continue;
				// same DFA state and same length
				if (distance >= queue[i].distance)
					// the distance is the same, or even longer, not of interset
					return;
			}
			queue.Add(new Node(previous, state, length, distance, error));
		}
	}
}