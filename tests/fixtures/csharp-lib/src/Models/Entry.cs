namespace Acme.Ledger.Models;

public record Entry(string Account, decimal Amount);

public enum EntryKind
{
    Debit,
    Credit,
}
