#include "tank.h"

void tank() {
  float direction = 0.0;
  while (1) {
    aim(direction);
    fire();
    direction += 0.1;
  }
}
