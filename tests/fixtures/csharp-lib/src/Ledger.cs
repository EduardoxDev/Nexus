using System;
using System.Collections.Generic;
using Acme.Ledger.Models;

namespace Acme.Ledger
{
    /// <summary>Livro-razão: class Fake {} não é declaração.</summary>
    public class Ledger
    {
        private readonly List<Entry> _entries = new();

        public decimal Balance { get; private set; }

        public Ledger()
        {
        }

        public void Post(Entry entry)
        {
            if (entry.Amount == 0)
            {
                throw new ArgumentException("lançamento vazio");
            }
            _entries.Add(entry);
            Balance += entry.Amount;
        }

        public IReadOnlyList<Entry> Entries => _entries;
    }
}
