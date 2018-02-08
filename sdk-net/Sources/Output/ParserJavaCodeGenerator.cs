/*******************************************************************************
 * Copyright (c) 2017 Association Cénotélie (cenotelie.fr)
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

using System.Collections.Generic;
using System.IO;
using System.Text;
using Hime.SDK.Grammars;

namespace Hime.SDK.Output
{
	/// <summary>
	/// Represents a generator for parser code for the Java platform
	/// </summary>
	public class ParserJavaCodeGenerator : Generator
	{
		/// <summary>
		/// The nmespace of the generated code
		/// </summary>
		private readonly string nmespace;
		/// <summary>
		/// The visibility modifier for the generated code
		/// </summary>
		private readonly Modifier modifier;
		/// <summary>
		/// The name of the generated lexer
		/// </summary>
		private readonly string name;
		/// <summary>
		/// Path to the automaton's binary resource
		/// </summary>
		private readonly string binResource;
		/// <summary>
		/// The grammar to generate a parser for
		/// </summary>
		private readonly Grammar grammar;
		/// <summary>
		/// The type of the parser to generate
		/// </summary>
		private readonly string parserType;
		/// <summary>
		/// The type of the automaton
		/// </summary>
		private readonly string automatonType;

		/// <summary>
		/// Initializes this code generator
		/// </summary>
		/// <param name="unit">The unit to generate code for</param>
		/// <param name="binResource">Path to the automaton's binary resource</param>
		public ParserJavaCodeGenerator(Unit unit, string binResource)
		{
			this.nmespace = Helper.GetNamespaceForJava(unit.Namespace == null ? unit.Grammar.Name : unit.Namespace);
			this.modifier = unit.Modifier;
			this.name = unit.Name;
			this.binResource = binResource;
			this.grammar = unit.Grammar;
			if (unit.Method == ParsingMethod.RNGLR1 || unit.Method == ParsingMethod.RNGLALR1)
			{
				this.parserType = "RNGLRParser";
				this.automatonType = "RNGLRAutomaton";
			}
			else
			{
				this.parserType = "LRkParser";
				this.automatonType = "LRkAutomaton";
			}
		}

		/// <summary>
		/// Writes a generated .Net file header
		/// </summary>
		/// <param name="writer">The writer to write to</param>
		private void WriteHeader(StreamWriter writer)
		{
			writer.WriteLine("/*");
			writer.WriteLine(" * WARNING: this file has been generated by");
			writer.WriteLine(" * Hime Parser Generator " + CompilationTask.Version);
			writer.WriteLine(" */");
		}

		/// <summary>
		/// Generates code for the specified file
		/// </summary>
		/// <param name="file">The target file to generate code in</param>
		public void Generate(string file)
		{
			StreamWriter writer = new StreamWriter(file, false, new UTF8Encoding(false));

			WriteHeader(writer);

			writer.WriteLine();
			writer.WriteLine("package " + nmespace + ";");
			writer.WriteLine();
			writer.WriteLine("import fr.cenotelie.hime.redist.SemanticAction;");
			writer.WriteLine("import fr.cenotelie.hime.redist.SemanticBody;");
			writer.WriteLine("import fr.cenotelie.hime.redist.Symbol;");
			writer.WriteLine("import fr.cenotelie.hime.redist.parsers.InitializationException;");
			writer.WriteLine("import fr.cenotelie.hime.redist.parsers." + automatonType + ";");
			writer.WriteLine("import fr.cenotelie.hime.redist.parsers." + parserType + ";");
			writer.WriteLine();
			writer.WriteLine("import java.util.Map;");
			writer.WriteLine();

			string mod = modifier == Modifier.Public ? "public " : "";

			writer.WriteLine("/**");
			writer.WriteLine(" * Represents a parser");
			writer.WriteLine(" */");
			writer.WriteLine(mod + "class " + name + "Parser extends " + parserType + " {");

			writer.WriteLine("    /**");
			writer.WriteLine("     * The automaton for this parser");
			writer.WriteLine("     */");
			writer.WriteLine("    private static final " + automatonType + " COMMON_AUTOMATON = " + automatonType + ".find(" + name + "Parser.class, \"" + binResource + "\");");

			GenerateCodeSymbols(writer);
			GenerateCodeVariables(writer);
			GenerateCodeVirtuals(writer);
			GenerateCodeActions(writer);
			GeneratorCodeConstructors(writer);

			writer.WriteLine("}");
			writer.Close();
		}

		/// <summary>
		/// Generates the code for the symbols
		/// </summary>
		/// <param name="stream">The output stream</param>
		private void GenerateCodeSymbols(StreamWriter stream)
		{
			stream.WriteLine("    /**");
			stream.WriteLine("     * Contains the constant IDs for the variables and virtuals in this parser");
			stream.WriteLine("     */");
			stream.WriteLine("    public static class ID {");
			foreach (Variable var in grammar.Variables)
			{
				if (var.Name.StartsWith(Grammar.PREFIX_GENERATED_VARIABLE))
					continue;
				stream.WriteLine("        /**");
				stream.WriteLine("         * The unique identifier for variable " + var.Name);
				stream.WriteLine("         */");
				stream.WriteLine("        public static final int VARIABLE_{0} = 0x{1};", Helper.ToUpperCase(var.Name), var.ID.ToString("X4"));
			}
			foreach (Virtual var in grammar.Virtuals)
			{
				stream.WriteLine("        /**");
				stream.WriteLine("         * The unique identifier for virtual " + var.Name);
				stream.WriteLine("         */");
				stream.WriteLine("        public static final int VIRTUAL_{0} = 0x{1};", Helper.ToUpperCase(var.Name), var.ID.ToString("X4"));
			}
			stream.WriteLine("    }");
		}

		/// <summary>
		/// Generates the code for the variables
		/// </summary>
		/// <param name="stream">The output stream</param>
		private void GenerateCodeVariables(StreamWriter stream)
		{
			stream.WriteLine("    /**");
			stream.WriteLine("     * The collection of variables matched by this parser");
			stream.WriteLine("     *");
			stream.WriteLine("     * The variables are in an order consistent with the automaton,");
			stream.WriteLine("     * so that variable indices in the automaton can be used to retrieve the variables in this table");
			stream.WriteLine("     */");
			stream.WriteLine("    private static final Symbol[] variables = {");
			bool first = true;
			foreach (Variable var in grammar.Variables)
			{
				if (!first)
					stream.WriteLine(", ");
				stream.Write("        ");
				stream.Write("new Symbol(0x" + var.ID.ToString("X4") + ", \"" + var.Name + "\")");
				first = false;
			}
			stream.WriteLine(" };");
		}

		/// <summary>
		/// Generates the code for the virtual symbols
		/// </summary>
		/// <param name="stream">The output stream</param>
		private void GenerateCodeVirtuals(StreamWriter stream)
		{
			stream.WriteLine("    /**");
			stream.WriteLine("     * The collection of virtuals matched by this parser");
			stream.WriteLine("     *");
			stream.WriteLine("     * The virtuals are in an order consistent with the automaton,");
			stream.WriteLine("     * so that virtual indices in the automaton can be used to retrieve the virtuals in this table");
			stream.WriteLine("     */");
			stream.WriteLine("    private static final Symbol[] virtuals = {");
			bool first = true;
			foreach (Virtual v in grammar.Virtuals)
			{
				if (!first)
					stream.WriteLine(", ");
				stream.Write("        ");
				stream.Write("new Symbol(0x" + v.ID.ToString("X4") + ", \"" + v.Name + "\")");
				first = false;
			}
			stream.WriteLine(" };");
		}

		/// <summary>
		/// Generates the code for the semantic actions
		/// </summary>
		/// <param name="stream">The output stream</param>
		private void GenerateCodeActions(StreamWriter stream)
		{
			if (grammar.Actions.Count == 0)
				return;
			stream.WriteLine("    /**");
			stream.WriteLine("     * Represents a set of semantic actions in this parser");
			stream.WriteLine("     */");
			stream.WriteLine("    public static class Actions {");
			foreach (Action action in grammar.Actions)
			{
				stream.WriteLine("        /**");
				stream.WriteLine("         * The " + action.Name + " semantic action");
				stream.WriteLine("         */");
				stream.WriteLine("        public void " + Helper.ToLowerCamelCase(action.Name) + "(Symbol head, SemanticBody body) { }");
			}
			stream.WriteLine();
			stream.WriteLine("    }");

			stream.WriteLine("    /**");
			stream.WriteLine("     * Represents a set of empty semantic actions (do nothing)");
			stream.WriteLine("     */");
			stream.WriteLine("    private static final Actions noActions = new Actions();");

			stream.WriteLine("    /**");
			stream.WriteLine("     * Gets the set of semantic actions in the form a table consistent with the automaton");
			stream.WriteLine("     *");
			stream.WriteLine("     * @param input A set of semantic actions");
			stream.WriteLine("     * @return A table of semantic actions");
			stream.WriteLine("     */");
			stream.WriteLine("    private static SemanticAction[] getUserActions(final Actions input) {");
			stream.WriteLine("        SemanticAction[] result = new SemanticAction[" + grammar.Actions.Count + "];");
			int i = 0;
			foreach (Action action in grammar.Actions)
			{
				stream.WriteLine("        result[" + i + "] = new SemanticAction() { @Override public void execute(Symbol head, SemanticBody body) { input." + Helper.ToLowerCamelCase(action.Name) + "(head, body); } };");
				i++;
			}
			stream.WriteLine("        return result;");
			stream.WriteLine("    }");

			stream.WriteLine("    /**");
			stream.WriteLine("     * Gets the set of semantic actions in the form a table consistent with the automaton");
			stream.WriteLine("     *");
			stream.WriteLine("     * @param input A set of semantic actions");
			stream.WriteLine("     * @return A table of semantic actions");
			stream.WriteLine("     */");
			stream.WriteLine("    private static SemanticAction[] getUserActions(Map<String, SemanticAction> input)");
			stream.WriteLine("    {");
			stream.WriteLine("        SemanticAction[] result = new SemanticAction[" + grammar.Actions.Count + "];");
			i = 0;
			foreach (Action action in grammar.Actions)
			{
				stream.WriteLine("        result[" + i + "] = input.get(\"" + action.Name + "\");");
				i++;
			}
			stream.WriteLine("        return result;");
			stream.WriteLine("    }");
		}

		/// <summary>
		/// Generates the code for the constructors
		/// </summary>
		/// <param name="stream">The output stream</param>
		private void GeneratorCodeConstructors(StreamWriter stream)
		{
			string ex = parserType.StartsWith("RNGLR") ? "throws InitializationException " : "";

			if (grammar.Actions.Count == 0)
			{
				stream.WriteLine("    /**");
				stream.WriteLine("     * Initializes a new instance of the parser");
				stream.WriteLine("     *");
				stream.WriteLine("     * @param lexer The input lexer");
				stream.WriteLine("     */");
				stream.WriteLine("    public " + name + "Parser(" + name + "Lexer lexer) " + ex + "{");
				stream.WriteLine("        super(COMMON_AUTOMATON, variables, virtuals, null, lexer);");
				stream.WriteLine("    }");
			}
			else
			{
				stream.WriteLine("    /**");
				stream.WriteLine("     * Initializes a new instance of the parser");
				stream.WriteLine("     *");
				stream.WriteLine("     * @param lexer The input lexer");
				stream.WriteLine("     */");
				stream.WriteLine("    public " + name + "Parser(" + name + "Lexer lexer) " + ex + "{");
				stream.WriteLine("        super(COMMON_AUTOMATON, variables, virtuals, getUserActions(noActions), lexer);");
				stream.WriteLine("    }");

				stream.WriteLine("    /**");
				stream.WriteLine("     * Initializes a new instance of the parser");
				stream.WriteLine("     *");
				stream.WriteLine("     * @param lexer The input lexer");
				stream.WriteLine("     * @param actions The set of semantic actions");
				stream.WriteLine("     */");
				stream.WriteLine("    public " + name + "Parser(" + name + "Lexer lexer, Actions actions) " + ex + "{");
				stream.WriteLine("        super(COMMON_AUTOMATON, variables, virtuals, getUserActions(actions), lexer);");
				stream.WriteLine("    }");

				stream.WriteLine("    /**");
				stream.WriteLine("     * Initializes a new instance of the parser");
				stream.WriteLine("     *");
				stream.WriteLine("     * @param lexer The input lexer");
				stream.WriteLine("     * @param actions The set of semantic actions");
				stream.WriteLine("     */");
				stream.WriteLine("    public " + name + "Parser(" + name + "Lexer lexer, Map<String, SemanticAction> actions) " + ex + "{");
				stream.WriteLine("        super(COMMON_AUTOMATON, variables, virtuals, getUserActions(actions), lexer);");
				stream.WriteLine("    }");
			}
		}
	}
}