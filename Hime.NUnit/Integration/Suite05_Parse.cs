﻿using System;
using System.Text;
using System.Reflection;
using System.Collections.Generic;
using NUnit.Framework;
using Hime.Parsers;
using Hime.Kernel.Reporting;
using Hime.Redist.Parsers;

namespace Hime.NUnit.Integration
{
    [TestFixture]
    public class Suite05_Parse : BaseTestSuite
    {
        private const string grammar0 = "public cf text grammar Test { options{ Axiom=\"S\"; } terminals{} rules{ S->'a'S'b'T|'c'T|'d'; T->'a'T|'b'S|'c'; } }";
        private const string grammar1 = "public cf text grammar Test { options{ Axiom=\"test\"; } terminals{} rules{ test->'x'*; } }";

        private void TestGrammar(string grammar, EParsingMethod method, string input)
        {
            Assert.IsFalse(CompileRaw(grammar, method).HasErrors, "Grammar compilation failed!");
            Assembly assembly = Build();
            bool errors = false;
            SyntaxTreeNode node = Parse(assembly, input, out errors);
            Assert.NotNull(node, "Failed to parse input!");
            Assert.IsFalse(errors, "Parsing errors!");
        }

        [Test]
        public void Test001_SimpleGrammar_LR0()
        {
            TestGrammar(grammar0, EParsingMethod.LR0, "adbc");
        }

        [Test]
        public void Test002_SimpleList_LR1()
        {
            TestGrammar(grammar1, EParsingMethod.LR1, "xxx");
        }

        [Test]
        public void Test003_SimpleList_LALR1()
        {
            TestGrammar(grammar1, EParsingMethod.LALR1, "xxx");
        }

        [Test]
        public void Test004_SimpleList_LRStar()
        {
            TestGrammar(grammar1, EParsingMethod.LRStar, "xxx");
        }
    }
}