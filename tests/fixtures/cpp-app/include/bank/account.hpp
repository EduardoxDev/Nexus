#pragma once

#include <string>
#include <vector>

namespace bank {

class Account {
public:
    explicit Account(std::string owner);
    long balance() const;
    void deposit(long cents);

private:
    std::string owner_;
    std::vector<long> history_;
};

}  // namespace bank
