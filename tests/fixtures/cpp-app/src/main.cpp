#include <iostream>
#include <bank/account.hpp>

int main() {
    bank::Account acc("Ana");
    acc.deposit(1500);
    std::cout << acc.balance() << "\n";
    return 0;
}
