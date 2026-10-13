namespace Acme.Ledger.Models;

public sealed class Account
{
    public const int MaxNameLength = 80;

    public string Name { get; init; } = "";

    public static bool IsValid(string name) => name.Length <= MaxNameLength;
}
