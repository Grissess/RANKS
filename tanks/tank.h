#ifndef TANK_H
#define TANK_H 1

#include <stdint.h>

extern uint64_t scan(float, float);
extern void fire();
extern void aim(float);
extern void turn(float);
extern float gpsx();
extern float gpsy();
extern void forward();
extern void explode();
extern void yield();

#endif
