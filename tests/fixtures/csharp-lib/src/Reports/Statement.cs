using System.Text;
using Acme.Ledger;
using static Acme.Ledger.Models.Account;

namespace Acme.Ledger.Reports
{
    internal static class Statement
    {
        public static string Render(Ledger ledger)
        {
            var sb = new StringBuilder();
            foreach (var e in ledger.Entries)
            {
                sb.AppendLine($"{e.Account}: {e.Amount}");
            }
            return sb.ToString();
        }
    }
}
