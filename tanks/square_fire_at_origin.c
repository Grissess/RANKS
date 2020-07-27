#include "tank.h"

inline float course_to(float x, float y) {
  float diffx = x - gpsx();
  float diffy = y - gpsy();
  return atan2_float(diffy, diffx);
}

inline float dist_to(float x, float y) {
  float diffx = x - gpsx();
  float diffy = y - gpsy();
  return hypot_float(diffx, diffy);
}

void tank() {
  post_string("greetings from a tank!");
  int max_fire_heat = DEATH_HEAT() - SHOOT_HEAT();
  int dest = 0;
  while (1) {
    float destx, desty;
    switch(dest) {
      case 0:
        destx = 100;
        desty = 0;
        break;
      case 1:
        destx = 100;
        desty = 100;
        break;
      case 2:
        destx = 0;
        desty = 100;
        break;
      case 3:
        destx = 0;
        desty = 0;
        break;
    }
    yield();
    post_string("navigating to (x, y):");
    post_float(destx);
    post_float(desty);
    float course = course_to(destx, desty);
    turn(course);
    while (dist_to(destx, desty) > TANK_VELOCITY()) {
      forward();
      if (temp() < max_fire_heat) {
        aim(atan2_float(-gpsy(), -gpsx()));
        fire();
      }
    }
    dest++;
    if (dest > 3) {
      dest = 0;
    }
  }
}
