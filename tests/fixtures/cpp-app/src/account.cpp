#include "bank/account.hpp"

#include <numeric>
#include <utility>

namespace bank {

Account::Account(std::string owner) : owner_(std::move(owner)) {}

long Account::balance() const {
    return std::accumulate(history_.begin(), history_.end(), 0L);
}

void Account::deposit(long cents) {
    if (cents <= 0) {
        return;
    }
    history_.push_back(cents);
}

}  // namespace bank
